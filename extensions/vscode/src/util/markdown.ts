/**
 * Markdown escaping for text rendered into hovers.
 *
 * Free of `vscode` imports so it can be exercised under `node --test`.
 */

/**
 * Neutralize markdown control characters.
 *
 * Values reaching a hover come from CLI output, which reflects
 * repository-controlled paths and config. Unescaped, a backtick closes an
 * inline code span and a fence closes a block, letting a repository inject
 * arbitrary markdown — remote images in particular leak IP and user-agent on
 * hover alone.
 */
export function escapeMarkdown(text: string): string {
  return text.replace(/[\\`*_{}[\]()#+\-.!|>~]/g, "\\$&");
}

/**
 * Strip control characters from a string destined for the clipboard.
 *
 * The "Copy Command" action copies a config-derived command line whose whole
 * purpose is to be pasted into a terminal; an embedded newline would execute on
 * paste rather than sit there to be read.
 */
export function stripControlChars(text: string): string {
  return text.replace(/[\x00-\x1F\x7F]/g, " ").trim();
}
