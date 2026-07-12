import * as path from "node:path";
import * as vscode from "vscode";
import type { CliClient } from "../cli/client";
import type { RunnableEntry } from "../cli/types";

export type RunnableTreeNode = FileNode | RunnableNode;

export class FileNode extends vscode.TreeItem {
  constructor(
    public readonly filePath: string,
    public readonly runnables: RunnableEntry[],
  ) {
    super(path.basename(filePath), vscode.TreeItemCollapsibleState.Collapsed);
    this.contextValue = "cargoRunnerFile";
    this.resourceUri = vscode.Uri.file(filePath);
    this.iconPath = new vscode.ThemeIcon("file-code");
    this.description = path.dirname(filePath);
    this.tooltip = filePath;
  }
}

export class RunnableNode extends vscode.TreeItem {
  constructor(public readonly entry: RunnableEntry) {
    super(entry.label, vscode.TreeItemCollapsibleState.None);
    this.contextValue = "cargoRunnerRunnable";
    this.iconPath = iconForKind(entry.kind);
    this.description = entry.command?.shell;
    this.tooltip = entry.command?.shell || entry.label;
    const line = (entry.scope?.start?.line ?? 0) + 1;
    this.command = {
      command: "vscode.open",
      title: "Open",
      arguments: [
        vscode.Uri.file(entry.file_path),
        {
          selection: new vscode.Range(line - 1, 0, line - 1, 0),
        },
      ],
    };
  }

  fileArg(): string {
    const line = (this.entry.scope?.start?.line ?? 0) + 1;
    return `${this.entry.file_path}:${line}`;
  }
}

function iconForKind(kind: RunnableEntry["kind"]): vscode.ThemeIcon {
  const key = Object.keys(kind)[0];
  switch (key) {
    case "Test":
    case "ModuleTests":
      return new vscode.ThemeIcon("beaker");
    case "Benchmark":
      return new vscode.ThemeIcon("dashboard");
    case "DocTest":
      return new vscode.ThemeIcon("book");
    case "Binary":
      return new vscode.ThemeIcon("play");
    default:
      return new vscode.ThemeIcon("symbol-method");
  }
}

export class RunnablesTreeProvider
  implements vscode.TreeDataProvider<RunnableTreeNode>
{
  private readonly _onDidChange = new vscode.EventEmitter<
    RunnableTreeNode | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChange.event;

  private cache: RunnableEntry[] = [];
  /** When true, scan whole workspace; else prefer active file. */
  workspaceMode = false;

  constructor(private readonly client: CliClient) {}

  refresh(): void {
    this._onDidChange.fire();
  }

  toggleWorkspaceMode(): void {
    this.workspaceMode = !this.workspaceMode;
    this.refresh();
  }

  getTreeItem(element: RunnableTreeNode): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: RunnableTreeNode): Promise<RunnableTreeNode[]> {
    if (!element) {
      try {
        const editor = vscode.window.activeTextEditor;
        if (
          !this.workspaceMode &&
          editor &&
          editor.document.languageId === "rust"
        ) {
          this.cache = await this.client.runnables(editor.document.uri.fsPath, {
            withCommands: true,
          });
        } else {
          this.cache = await this.client.runnables(undefined, {
            withCommands: true,
          });
        }
      } catch (e) {
        vscode.window.showErrorMessage(`Failed to load runnables: ${e}`);
        this.cache = [];
      }

      const byFile = new Map<string, RunnableEntry[]>();
      for (const r of this.cache) {
        const list = byFile.get(r.file_path) || [];
        list.push(r);
        byFile.set(r.file_path, list);
      }

      return [...byFile.entries()].map(
        ([filePath, runnables]) => new FileNode(filePath, runnables),
      );
    }

    if (element instanceof FileNode) {
      return element.runnables.map((r) => new RunnableNode(r));
    }

    return [];
  }
}
