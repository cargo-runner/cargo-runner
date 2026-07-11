import * as path from "node:path";
import * as vscode from "vscode";
import { BinaryManager } from "./binary/manager";
import { CliClient } from "./cli/client";
import { overrideAt, overrideAtCursor } from "./commands/override";
import { debugFileArg, runAtCursor, runFileArg } from "./commands/run";
import { showDebugInfo } from "./debug/breakpoint";
import { CargoRunnerCodeLensProvider } from "./providers/codeLens";
import { executeAsTask, registerTaskProvider } from "./providers/taskProvider";
import { OverrideItem, OverridesTreeProvider } from "./views/overridesTree";
import {
  RunnableNode,
  RunnablesTreeProvider,
} from "./views/runnablesTree";

let output: vscode.OutputChannel;

export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  output = vscode.window.createOutputChannel("Cargo Runner");
  context.subscriptions.push(output);
  output.appendLine("Cargo Runner extension activating…");

  const binaryManager = new BinaryManager(context, output);
  const client = new CliClient(() => binaryManager.ensureBinary(), output);

  binaryManager.ensureBinary().catch((e) => {
    output.appendLine(`Binary ensure deferred: ${e}`);
  });

  context.subscriptions.push(registerTaskProvider(context));

  const runnablesTree = new RunnablesTreeProvider(client);
  const overridesTree = new OverridesTreeProvider(client);
  const codeLens = new CargoRunnerCodeLensProvider(client, output);

  context.subscriptions.push(
    vscode.window.registerTreeDataProvider(
      "cargoRunner.runnables",
      runnablesTree,
    ),
    vscode.window.registerTreeDataProvider(
      "cargoRunner.overrides",
      overridesTree,
    ),
    vscode.languages.registerCodeLensProvider({ language: "rust" }, codeLens),
  );

  const watcher = vscode.workspace.createFileSystemWatcher(
    "**/{*.rs,Cargo.toml,.cargo-runner.json,BUILD.bazel}",
  );
  const refreshAll = () => {
    runnablesTree.refresh();
    overridesTree.refresh();
    codeLens.refresh();
  };
  watcher.onDidChange(refreshAll);
  watcher.onDidCreate(refreshAll);
  watcher.onDidDelete(refreshAll);
  context.subscriptions.push(watcher);

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document.languageId === "rust") {
        codeLens.invalidateDocument(e.document.uri);
      }
    }),
  );

  const status = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    50,
  );
  status.text = "$(play) Cargo Runner";
  status.tooltip = "Run at cursor (Cmd+R)";
  status.command = "cargoRunner.run";
  status.show();
  context.subscriptions.push(status);

  const wrap = (fn: () => Promise<void>) => async () => {
    try {
      await fn();
    } catch (e) {
      output.appendLine(`error: ${e}`);
      vscode.window.showErrorMessage(`Cargo Runner: ${e}`);
    }
  };

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "cargoRunner.run",
      wrap(() => runAtCursor(binaryManager, client, output)),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.override",
      wrap(async () => {
        await overrideAtCursor(binaryManager, client, output);
        overridesTree.refresh();
        codeLens.refresh();
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.runAt",
      async (fileArg: string) => {
        try {
          await runFileArg(binaryManager, client, output, fileArg);
        } catch (e) {
          vscode.window.showErrorMessage(`Cargo Runner: ${e}`);
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.debugAt",
      async (fileArg: string) => {
        try {
          await debugFileArg(binaryManager, client, output, fileArg);
        } catch (e) {
          vscode.window.showErrorMessage(`Cargo Runner: ${e}`);
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.overrideAt",
      async (fileArg: string) => {
        try {
          await overrideAt(binaryManager, client, output, fileArg);
          overridesTree.refresh();
          codeLens.refresh();
        } catch (e) {
          vscode.window.showErrorMessage(`Cargo Runner: ${e}`);
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.runTreeItem",
      async (item: RunnableNode) => {
        if (item instanceof RunnableNode) {
          await vscode.commands.executeCommand(
            "cargoRunner.runAt",
            item.fileArg(),
          );
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.overrideTreeItem",
      async (item: RunnableNode) => {
        if (item instanceof RunnableNode) {
          await vscode.commands.executeCommand(
            "cargoRunner.overrideAt",
            item.fileArg(),
          );
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.copyCommand",
      async (item: RunnableNode) => {
        if (!(item instanceof RunnableNode)) {
          return;
        }
        const shell =
          item.entry.command?.shell ||
          (await client
            .dryRun(item.fileArg())
            .then((d) => d.shell)
            .catch(() => item.entry.label));
        await vscode.env.clipboard.writeText(shell);
        vscode.window.showInformationMessage("Command copied to clipboard");
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.deleteOverride",
      async (item: OverrideItem) => {
        if (!(item instanceof OverrideItem)) {
          return;
        }
        const file = item.fileArg();
        if (!file) {
          vscode.window.showErrorMessage("No file path on override");
          return;
        }
        const fn = item.entry.override.match?.function_name;
        // Prefer path; line unknown — file-level remove uses match on file_path
        const fileArg = file;
        const confirm = await vscode.window.showWarningMessage(
          `Remove override for ${fn || path.basename(file)}?`,
          { modal: true },
          "Remove",
        );
        if (confirm !== "Remove") {
          return;
        }
        try {
          await client.setOverride(fileArg, ["-"]);
          vscode.window.showInformationMessage("Override removed");
          overridesTree.refresh();
        } catch (e) {
          vscode.window.showErrorMessage(`Remove failed: ${e}`);
        }
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.runOverride",
      async (item: OverrideItem) => {
        if (!(item instanceof OverrideItem)) {
          return;
        }
        const file = item.fileArg();
        if (!file) {
          return;
        }
        await vscode.commands.executeCommand("cargoRunner.runAt", file);
      },
    ),
    vscode.commands.registerCommand(
      "cargoRunner.selectRunnable",
      wrap(async () => {
        const editor = vscode.window.activeTextEditor;
        const file =
          editor?.document.languageId === "rust"
            ? editor.document.uri.fsPath
            : undefined;
        const items = await client.runnables(file, { withCommands: true });
        if (items.length === 0) {
          vscode.window.showInformationMessage("No runnables found");
          return;
        }
        const picked = await vscode.window.showQuickPick(
          items.map((r) => ({
            label: r.label,
            description: r.command?.shell,
            detail: `${r.file_path}:${(r.scope?.start?.line ?? 0) + 1}`,
            entry: r,
          })),
          { placeHolder: "Select a runnable" },
        );
        if (!picked) {
          return;
        }
        const line = (picked.entry.scope?.start?.line ?? 0) + 1;
        await runFileArg(
          binaryManager,
          client,
          output,
          `${picked.entry.file_path}:${line}`,
        );
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.setupBinary",
      wrap(async () => {
        const config = vscode.workspace.getConfiguration("cargoRunner");
        const choice = await vscode.window.showQuickPick(
          [
            {
              label: "$(cloud-download) Auto-download",
              value: "auto",
              description: "Download prebuilt binary into extension storage",
            },
            {
              label: "$(terminal) Use PATH",
              value: "path",
              description: "cargo-runner from system PATH",
            },
            {
              label: "$(folder) Custom path",
              value: "custom",
            },
          ],
          { title: "Cargo Runner Binary Setup" },
        );
        if (!choice) {
          return;
        }
        if (choice.value === "auto") {
          await config.update("path", "", vscode.ConfigurationTarget.Global);
          try {
            const p = await binaryManager.updateBinary();
            vscode.window.showInformationMessage(`Cargo Runner ready: ${p}`);
          } catch (e) {
            vscode.window.showErrorMessage(
              `Download failed: ${e}. Try: cargo binstall cargo-runner-cli`,
            );
          }
        } else if (choice.value === "path") {
          await config.update(
            "path",
            "cargo-runner",
            vscode.ConfigurationTarget.Global,
          );
          vscode.window.showInformationMessage(
            "Using cargo-runner from PATH",
          );
        } else {
          const custom = await vscode.window.showInputBox({
            prompt: "Full path to cargo-runner binary",
          });
          if (custom) {
            await config.update(
              "path",
              custom,
              vscode.ConfigurationTarget.Global,
            );
          }
        }
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.updateBinary",
      wrap(async () => {
        const p = await binaryManager.updateBinary();
        vscode.window.showInformationMessage(`Updated: ${p}`);
      }),
    ),
    vscode.commands.registerCommand("cargoRunner.refreshRunnables", () =>
      runnablesTree.refresh(),
    ),
    vscode.commands.registerCommand("cargoRunner.refreshOverrides", () =>
      overridesTree.refresh(),
    ),
    vscode.commands.registerCommand("cargoRunner.toggleWorkspaceScan", () => {
      runnablesTree.toggleWorkspaceMode();
      vscode.window.showInformationMessage(
        runnablesTree.workspaceMode
          ? "Runnables: workspace scan"
          : "Runnables: active file",
      );
    }),
    vscode.commands.registerCommand(
      "cargoRunner.init",
      wrap(async () => {
        const binary = await binaryManager.ensureBinary();
        const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        const mode = await vscode.window.showQuickPick(
          [
            { label: "Standard init", value: "init", args: ["init"] },
            {
              label: "Bazel init",
              value: "bazel",
              args: ["init", "--bazel"],
            },
          ],
          { placeHolder: "Initialize cargo-runner config" },
        );
        if (!mode) {
          return;
        }
        await executeAsTask(binary, mode.args, {
          cwd,
          label: `Cargo Runner: ${mode.label}`,
        });
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.clean",
      wrap(async () => {
        const binary = await binaryManager.ensureBinary();
        const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        await executeAsTask(binary, ["clean"], {
          cwd,
          label: "Cargo Runner: clean",
        });
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.watch",
      wrap(async () => {
        const binary = await binaryManager.ensureBinary();
        const editor = vscode.window.activeTextEditor;
        const folder =
          (editor &&
            vscode.workspace.getWorkspaceFolder(editor.document.uri)) ||
          vscode.workspace.workspaceFolders?.[0];
        const cwd = folder?.uri.fsPath;
        const mode = await vscode.window.showQuickPick(
          [
            { label: "Watch + build", args: ["watch"] },
            { label: "Watch + run", args: ["watch", "--run"] },
            { label: "Watch + test", args: ["watch", "--test"] },
          ],
          { placeHolder: "Watch mode" },
        );
        if (!mode) {
          return;
        }
        await executeAsTask(binary, mode.args, {
          cwd,
          label: `Cargo Runner: ${mode.label}`,
          isBackground: true,
        });
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.context",
      wrap(async () => {
        const editor = vscode.window.activeTextEditor;
        const fileArg =
          editor?.document.languageId === "rust"
            ? `${editor.document.uri.fsPath}:${editor.selection.active.line + 1}`
            : undefined;
        const ctx = await client.context(fileArg);
        output.appendLine(JSON.stringify(ctx, null, 2));
        output.show(true);
        status.text = `$(play) CR · ${ctx.build_system}`;
      }),
    ),
    vscode.commands.registerCommand("cargoRunner.showOutput", () => {
      output.show(true);
    }),
    vscode.commands.registerCommand("cargoRunner.openRunnables", async () => {
      await vscode.commands.executeCommand(
        "workbench.view.extension.cargo-runner",
      );
    }),
    vscode.commands.registerCommand(
      "cargoRunner.showDebugInfo",
      wrap(async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== "rust") {
          vscode.window.showErrorMessage("Open a Rust file first");
          return;
        }
        await showDebugInfo(editor.document, editor.selection.active);
      }),
    ),
    vscode.commands.registerCommand(
      "cargoRunner.toggleBreakpointDetection",
      async () => {
        const config = vscode.workspace.getConfiguration("cargoRunner");
        const current = config.get<boolean>(
          "enableBreakpointDetection",
          true,
        );
        await config.update(
          "enableBreakpointDetection",
          !current,
          vscode.ConfigurationTarget.Global,
        );
        vscode.window.showInformationMessage(
          `Breakpoint detection ${!current ? "enabled" : "disabled"}`,
        );
      },
    ),
  );

  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((ed) => {
      runnablesTree.refresh();
      if (ed?.document.languageId === "rust") {
        client
          .context(`${ed.document.uri.fsPath}:${ed.selection.active.line + 1}`)
          .then((ctx) => {
            status.text = `$(play) CR · ${ctx.build_system}`;
            status.tooltip = `${ctx.file_kind} · ${ctx.runnable_kind || "—"} · Cmd+R to run`;
          })
          .catch(() => {
            status.text = "$(play) Cargo Runner";
          });
      }
    }),
  );

  output.appendLine("Cargo Runner extension activated");
}

export function deactivate(): void {
  // nothing
}
