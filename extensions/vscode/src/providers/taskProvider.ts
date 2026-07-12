import * as vscode from "vscode";
import * as path from "node:path";

export interface CargoRunnerTaskDefinition extends vscode.TaskDefinition {
  type: "cargo-runner";
  args: string[];
  cwd?: string;
}

const LONG_RUNNING = [
  "serve",
  "watch",
  "dev",
  "dx serve",
  "leptos watch",
  "tauri dev",
  "trunk serve",
];

export function isLongRunning(shell: string): boolean {
  const lower = shell.toLowerCase();
  return LONG_RUNNING.some((p) => lower.includes(p));
}

export function registerTaskProvider(
  _context: vscode.ExtensionContext,
): vscode.Disposable {
  return vscode.tasks.registerTaskProvider("cargo-runner", {
    provideTasks: () => [],
    resolveTask: (task: vscode.Task) => {
      const def = task.definition as CargoRunnerTaskDefinition;
      return buildTask(def.args, {
        cwd: def.cwd,
        label: task.name,
        binary: "cargo-runner",
      });
    },
  });
}

export async function executeAsTask(
  binary: string,
  args: string[],
  options: {
    cwd?: string;
    label?: string;
    env?: Record<string, string>;
    isBackground?: boolean;
  },
): Promise<vscode.TaskExecution> {
  const task = buildTask(args, {
    ...options,
    binary,
  });

  // If same task already running, offer choices
  const existing = vscode.tasks.taskExecutions.find(
    (t) => t.task.name === task.name && t.task.source === "Cargo Runner",
  );
  if (existing) {
    const choice = await vscode.window.showQuickPick(
      [
        { label: "Show Task", value: "show" },
        { label: "Start New", value: "new" },
        { label: "Cancel", value: "cancel" },
      ],
      { placeHolder: "This Cargo Runner task is already running" },
    );
    if (!choice || choice.value === "cancel") {
      throw new Error("Cancelled");
    }
    if (choice.value === "show") {
      // Best-effort: focus terminal
      await vscode.commands.executeCommand("workbench.action.tasks.showTasks");
      return existing;
    }
  }

  return vscode.tasks.executeTask(task);
}

function buildTask(
  args: string[],
  options: {
    binary: string;
    cwd?: string;
    label?: string;
    env?: Record<string, string>;
    isBackground?: boolean;
  },
): vscode.Task {
  const def: CargoRunnerTaskDefinition = {
    type: "cargo-runner",
    args,
    cwd: options.cwd,
  };

  const shellArgs = args.map((a) =>
    a.includes(" ") && !a.startsWith('"') ? `"${a}"` : a,
  );
  const execution = new vscode.ShellExecution(options.binary, shellArgs, {
    cwd: options.cwd,
    env: options.env,
  });

  const label =
    options.label ||
    `Cargo Runner: ${args.slice(0, 3).join(" ")}${args.length > 3 ? "…" : ""}`;

  const task = new vscode.Task(
    def,
    vscode.TaskScope.Workspace,
    label,
    "Cargo Runner",
    execution,
    ["$rustc"],
  );

  task.presentationOptions = {
    reveal: vscode.TaskRevealKind.Always,
    panel: vscode.TaskPanelKind.Dedicated,
    clear: false,
    showReuseMessage: false,
  };

  if (options.isBackground) {
    task.isBackground = true;
    task.presentationOptions.reveal = vscode.TaskRevealKind.Always;
  }

  return task;
}

export function fileLabel(filePath: string): string {
  return path.basename(filePath);
}
