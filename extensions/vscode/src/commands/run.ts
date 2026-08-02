import * as path from "node:path";
import * as vscode from "vscode";
import type { BinaryManager } from "../binary/manager";
import type { CliClient } from "../cli/client";
import { tryDebugAtCursor } from "../debug/breakpoint";
import { executeAsTask, isLongRunning } from "../providers/taskProvider";

/** Live run terminals; exited ones are disposed on the next run. */
const terminals: vscode.Terminal[] = [];

export async function runAtCursor(
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
  const fileArg = `${filePath}:${line}`;

  const config = vscode.workspace.getConfiguration("cargoRunner");
  if (config.get<boolean>("enableBreakpointDetection", true)) {
    try {
      const debugged = await tryDebugAtCursor(
        editor.document,
        editor.selection.active,
        output,
      );
      if (debugged) {
        return;
      }
    } catch (e) {
      output.appendLine(`Debug handoff failed, falling back to run: ${e}`);
    }
  }

  await runFileArg(binaryManager, client, output, fileArg);
}

/** Run a specific `path:line` (CodeLens / tree / QuickPick). */
export async function runFileArg(
  binaryManager: BinaryManager,
  client: CliClient,
  output: vscode.OutputChannel,
  fileArg: string,
  options?: { skipDebug?: boolean },
): Promise<void> {
  void options;
  const filePath = fileArg.replace(/:\d+$/, "");
  const lineMatch = fileArg.match(/:(\d+)$/);
  const line = lineMatch ? lineMatch[1] : "?";
  const cwd =
    vscode.workspace.getWorkspaceFolder(vscode.Uri.file(filePath))?.uri
      .fsPath || path.dirname(filePath);

  const config = vscode.workspace.getConfiguration("cargoRunner");
  const binary = await binaryManager.ensureBinary();
  const useTask = config.get<boolean>("useTaskRunner", true);

  let shell = `cargo-runner run ${fileArg}`;
  let workDir = cwd;
  try {
    const dry = await client.dryRun(fileArg, cwd);
    shell = dry.shell;
    if (dry.cwd) {
      workDir = dry.cwd;
    }
    output.appendLine(`Resolved: ${shell}`);
  } catch (e) {
    output.appendLine(`Dry-run failed (will still try run): ${e}`);
  }

  const args = ["run", fileArg];
  const background = isLongRunning(shell);

  if (useTask) {
    try {
      await executeAsTask(binary, args, {
        cwd: workDir,
        label: `Cargo Runner: ${path.basename(filePath)}:${line}`,
        isBackground: background,
      });
      return;
    } catch (e) {
      if (String(e).includes("Cancelled")) {
        return;
      }
      output.appendLine(`Task failed, using terminal: ${e}`);
    }
  }

  await runInTerminal(
    binary,
    args,
    workDir,
    config.get<boolean>("showOutput", true),
  );
}

/** Open file at line then attempt debug handoff, else run. */
export async function debugFileArg(
  binaryManager: BinaryManager,
  client: CliClient,
  output: vscode.OutputChannel,
  fileArg: string,
): Promise<void> {
  const filePath = fileArg.replace(/:\d+$/, "");
  const lineMatch = fileArg.match(/:(\d+)$/);
  const line0 = lineMatch ? Math.max(0, parseInt(lineMatch[1], 10) - 1) : 0;

  const doc = await vscode.workspace.openTextDocument(filePath);
  const editor = await vscode.window.showTextDocument(doc);
  const pos = new vscode.Position(line0, 0);
  editor.selection = new vscode.Selection(pos, pos);
  editor.revealRange(new vscode.Range(pos, pos));

  try {
    const debugged = await tryDebugAtCursor(doc, pos, output);
    if (debugged) {
      return;
    }
  } catch (e) {
    output.appendLine(`Debug failed: ${e}`);
  }

  await runFileArg(binaryManager, client, output, fileArg);
}

/**
 * Fallback run path, used when the task runner is disabled or fails.
 *
 * The terminal is created with `shellPath`/`shellArgs`, so VS Code execs the
 * binary directly with an argv array and no shell is involved. The previous
 * approach — building a command line and calling `sendText` — required quoting
 * argv entries that carry workspace-controlled data (file paths, resolved
 * target names), and quoting for an unknown interactive shell is not something
 * that can be done reliably: `process.platform` does not identify the terminal's
 * configured profile, so the same string is safe in one shell and executable in
 * another.
 *
 * Each run gets its own terminal, since a terminal bound to a process cannot
 * accept a second command.
 */
async function runInTerminal(
  binary: string,
  args: string[],
  cwd: string,
  show: boolean,
): Promise<void> {
  // Reap every terminal whose process already exited, but never kill a run the
  // user may still be watching. Tracking the whole set matters because a
  // terminal bound to a process cannot be reused, so each run makes a new one —
  // holding only the most recent handle would strand the earlier ones.
  for (let i = terminals.length - 1; i >= 0; i--) {
    if (terminals[i].exitStatus !== undefined) {
      terminals[i].dispose();
      terminals.splice(i, 1);
    }
  }

  const terminal = vscode.window.createTerminal({
    name: "Cargo Runner",
    cwd,
    shellPath: binary,
    shellArgs: args,
  });
  terminals.push(terminal);
  if (show) {
    terminal.show();
  }
}
