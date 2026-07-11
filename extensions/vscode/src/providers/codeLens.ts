import * as vscode from "vscode";
import type { CliClient } from "../cli/client";
import type { RunnableEntry } from "../cli/types";

/**
 * CodeLens: ▶ Run · Debug · ⚙ Override above each cargo-runner runnable.
 */
export class CargoRunnerCodeLensProvider implements vscode.CodeLensProvider {
  private readonly _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChangeCodeLenses = this._onDidChange.event;

  private cache = new Map<string, { version: number; lenses: vscode.CodeLens[] }>();
  private debounceTimers = new Map<string, NodeJS.Timeout>();

  constructor(
    private readonly client: CliClient,
    private readonly output: vscode.OutputChannel,
  ) {}

  refresh(): void {
    this.cache.clear();
    this._onDidChange.fire();
  }

  /** Debounced invalidate for document changes. */
  invalidateDocument(uri: vscode.Uri, delayMs = 300): void {
    const key = uri.toString();
    const existing = this.debounceTimers.get(key);
    if (existing) {
      clearTimeout(existing);
    }
    this.debounceTimers.set(
      key,
      setTimeout(() => {
        this.cache.delete(key);
        this._onDidChange.fire();
        this.debounceTimers.delete(key);
      }, delayMs),
    );
  }

  async provideCodeLenses(
    document: vscode.TextDocument,
    token: vscode.CancellationToken,
  ): Promise<vscode.CodeLens[]> {
    const config = vscode.workspace.getConfiguration("cargoRunner");
    if (!config.get<boolean>("enableCodeLens", true)) {
      return [];
    }
    if (document.languageId !== "rust") {
      return [];
    }

    const key = document.uri.toString();
    const cached = this.cache.get(key);
    if (cached && cached.version === document.version) {
      return cached.lenses;
    }

    try {
      // Prefer unsaved buffer path on disk — CLI reads filesystem.
      // If dirty, still use fsPath; user should save for accurate line maps.
      const entries = await this.client.runnables(document.uri.fsPath, {
        withCommands: false,
        cwd:
          vscode.workspace.getWorkspaceFolder(document.uri)?.uri.fsPath ||
          undefined,
      });

      if (token.isCancellationRequested) {
        return [];
      }

      const lenses = buildLenses(document, entries);
      this.cache.set(key, { version: document.version, lenses });
      return lenses;
    } catch (e) {
      this.output.appendLine(`CodeLens: ${e}`);
      return [];
    }
  }
}

function buildLenses(
  document: vscode.TextDocument,
  entries: RunnableEntry[],
): vscode.CodeLens[] {
  const lenses: vscode.CodeLens[] = [];
  const seenLines = new Set<number>();

  for (const entry of entries) {
    // CLI scope lines are 0-based (see Runnable.scope in core).
    const startLine = Math.max(0, entry.scope?.start?.line ?? 0);
    if (startLine >= document.lineCount) {
      continue;
    }
    // Dedupe stacked lenses on same line (module + test)
    if (seenLines.has(startLine)) {
      // Still allow if kinds differ? Keep one group per line for cleanliness.
      continue;
    }
    seenLines.add(startLine);

    const range = new vscode.Range(startLine, 0, startLine, 0);
    const filePath = entry.file_path || document.uri.fsPath;
    const line1 = startLine + 1;
    const fileArg = `${filePath}:${line1}`;

    lenses.push(
      new vscode.CodeLens(range, {
        title: "▶ Run",
        tooltip: entry.command?.shell || entry.label,
        command: "cargoRunner.runAt",
        arguments: [fileArg],
      }),
    );
    lenses.push(
      new vscode.CodeLens(range, {
        title: "Debug",
        tooltip: "Debug via rust-analyzer when possible",
        command: "cargoRunner.debugAt",
        arguments: [fileArg],
      }),
    );
    lenses.push(
      new vscode.CodeLens(range, {
        title: "⚙ Override",
        tooltip: "Set override tokens for this runnable",
        command: "cargoRunner.overrideAt",
        arguments: [fileArg],
      }),
    );
  }

  return lenses;
}
