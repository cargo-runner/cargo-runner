import * as path from "node:path";
import * as vscode from "vscode";
import type { BinaryManager } from "../binary/manager";
import type { CliClient } from "../cli/client";
import { executeAsTask } from "../providers/taskProvider";

const TOKEN_HELP =
  "Tokens: @cmd.sub  +channel  KEY=val  /test-args  --flags  |  @ append  !! clear  !env";

/**
 * Cmd+Shift+R — prompt for override tokens at cursor.
 */
export async function overrideAtCursor(
  binaryManager: BinaryManager,
  client: CliClient,
  output: vscode.OutputChannel,
): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "rust") {
    vscode.window.showErrorMessage("Cargo Runner: open a Rust file first");
    return;
  }

  const filePath = editor.document.uri.fsPath;
  const line = editor.selection.active.line + 1;
  await overrideAt(binaryManager, client, output, `${filePath}:${line}`);
}

/** Override for an explicit file:line (CodeLens / tree). */
export async function overrideAt(
  binaryManager: BinaryManager,
  client: CliClient,
  output: vscode.OutputChannel,
  fileArg: string,
  prefill?: string,
): Promise<void> {
  const filePath = fileArg.replace(/:\d+$/, "");
  const lineMatch = fileArg.match(/:(\d+)$/);
  const line = lineMatch ? lineMatch[1] : "1";
  const cwd =
    vscode.workspace.getWorkspaceFolder(vscode.Uri.file(filePath))?.uri
      .fsPath || path.dirname(filePath);

  // Navigate to target so user sees context
  try {
    const doc = await vscode.workspace.openTextDocument(filePath);
    const editor = await vscode.window.showTextDocument(doc);
    const line0 = Math.max(0, parseInt(line, 10) - 1);
    const pos = new vscode.Position(line0, 0);
    editor.selection = new vscode.Selection(pos, pos);
  } catch {
    // file may not open; still try override
  }

  const tokensInput = await vscode.window.showInputBox({
    title: "Cargo Runner Override",
    prompt: TOKEN_HELP,
    placeHolder: "e.g. @dx.serve --release RUST_LOG=debug /--nocapture",
    value: prefill || "",
    ignoreFocusOut: true,
    validateInput: (value) => {
      if (value.trim() === "@.") {
        return "Incomplete @cmd.sub token";
      }
      return null;
    },
  });

  if (tokensInput === undefined) {
    return;
  }

  const tokens = tokenize(tokensInput.trim());
  const binary = await binaryManager.ensureBinary();

  try {
    const dry = await client.dryRun(fileArg, cwd);
    output.appendLine(`Before override: ${dry.shell}`);
  } catch (e) {
    output.appendLine(`Dry-run before override: ${e}`);
  }

  if (tokens.length === 0) {
    await executeAsTask(binary, ["run", fileArg], {
      cwd,
      label: `Cargo Runner: ${path.basename(filePath)}:${line}`,
    });
    return;
  }

  if (tokens.length === 1 && (tokens[0] === "-" || tokens[0] === "!!")) {
    await client.setOverride(fileArg, tokens, cwd);
    vscode.window.showInformationMessage("Cargo Runner: override removed");
    return;
  }

  const action = await vscode.window.showQuickPick(
    [
      {
        label: "$(save) Save & Run",
        description: "Write override to .cargo-runner.json then run",
        value: "save-run",
      },
      {
        label: "$(eye) Preview only",
        description: "Save override and show dry-run JSON",
        value: "preview",
      },
      {
        label: "$(check) Save only",
        description: "Write override without running",
        value: "save",
      },
    ],
    { placeHolder: "What should we do with this override?" },
  );

  if (!action) {
    return;
  }

  try {
    await client.setOverride(fileArg, tokens, cwd);
    output.appendLine(`Override saved: ${tokens.join(" ")}`);
  } catch (e) {
    vscode.window.showErrorMessage(`Failed to save override: ${e}`);
    return;
  }

  if (action.value === "save") {
    vscode.window.showInformationMessage("Cargo Runner: override saved");
    return;
  }

  if (action.value === "preview") {
    try {
      const dry = await client.dryRun(fileArg, cwd);
      output.appendLine(`After override: ${JSON.stringify(dry, null, 2)}`);
      output.show(true);
      vscode.window.showInformationMessage(`Preview: ${dry.shell}`);
    } catch (e) {
      vscode.window.showErrorMessage(`Preview failed: ${e}`);
    }
    return;
  }

  await executeAsTask(binary, ["run", fileArg], {
    cwd,
    label: `Cargo Runner: ${path.basename(filePath)}:${line}`,
  });
}

/** Split like a shell while keeping KEY=value and quoted strings. */
export function tokenize(input: string): string[] {
  if (!input) {
    return [];
  }
  return (
    input
      .match(/(?:[^\s"]+|"[^"]*")+/g)
      ?.map((t) => t.replace(/^"|"$/g, "")) ?? []
  );
}
