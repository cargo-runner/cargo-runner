import * as path from "node:path";
import * as vscode from "vscode";
import type { CliClient } from "../cli/client";
import type { OverrideListEntry } from "../cli/types";

export class OverrideItem extends vscode.TreeItem {
  constructor(public readonly entry: OverrideListEntry) {
    const match = entry.override.match || {};
    const label =
      match.function_name || path.basename(match.file_path || "override");
    super(label, vscode.TreeItemCollapsibleState.None);
    this.contextValue = "cargoRunnerOverride";
    this.iconPath = new vscode.ThemeIcon("settings-gear");
    this.description = summarize(entry);
    // Every value here comes from CLI output, which reflects repository-
    // controlled paths and config. Built with the append* helpers rather than
    // string concatenation: a backtick in a path used to close the inline code
    // span, and a fence inside the JSON used to close the block, letting a
    // repository inject arbitrary markdown into the hover — remote images in
    // particular leak IP and user-agent on hover alone. `isTrusted` stays at
    // its default of false, so `command:` links were never executable.
    const tooltip = new vscode.MarkdownString();
    tooltip.appendMarkdown(`**${escapeMarkdown(label)}**\n\n`);
    tooltip.appendMarkdown("File: ");
    tooltip.appendCodeblock(match.file_path || "?", "text");
    tooltip.appendMarkdown("Config: ");
    tooltip.appendCodeblock(entry.config_path, "text");
    tooltip.appendCodeblock(JSON.stringify(entry.override, null, 2), "json");
    this.tooltip = tooltip;
    if (match.file_path) {
      this.command = {
        command: "vscode.open",
        title: "Open",
        arguments: [vscode.Uri.file(match.file_path)],
      };
    }
  }

  fileArg(): string | undefined {
    const match = this.entry.override.match;
    if (!match?.file_path) {
      return undefined;
    }
    // Line is not always stored; use function name path for override show
    return match.file_path;
  }
}

function summarize(entry: OverrideListEntry): string {
  const parts: string[] = [];
  const cargo = entry.override.cargo as Record<string, unknown> | undefined;
  if (cargo?.command) {
    parts.push(String(cargo.command));
  }
  if (cargo?.subcommand) {
    parts.push(String(cargo.subcommand));
  }
  if (Array.isArray(cargo?.extra_args)) {
    parts.push((cargo.extra_args as string[]).join(" "));
  }
  if (cargo?.extra_env && typeof cargo.extra_env === "object") {
    parts.push(
      Object.entries(cargo.extra_env as Record<string, string>)
        .map(([k, v]) => `${k}=${v}`)
        .join(" "),
    );
  }
  return parts.filter(Boolean).join(" ") || "override";
}

export class OverridesTreeProvider
  implements vscode.TreeDataProvider<OverrideItem>
{
  private readonly _onDidChange = new vscode.EventEmitter<
    OverrideItem | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChange.event;

  constructor(private readonly client: CliClient) {}

  refresh(): void {
    this._onDidChange.fire();
  }

  getTreeItem(element: OverrideItem): vscode.TreeItem {
    return element;
  }

  async getChildren(): Promise<OverrideItem[]> {
    try {
      const list = await this.client.listOverrides();
      return list.map((e) => new OverrideItem(e));
    } catch (e) {
      console.error(e);
      return [];
    }
  }
}

/** Neutralize markdown control characters in text rendered into a hover. */
export function escapeMarkdown(text: string): string {
  return text.replace(/[\\`*_{}[\]()#+\-.!|>~]/g, "\\$&");
}
