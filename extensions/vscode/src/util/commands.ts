/**
 * Command-classification helpers.
 *
 * Free of `vscode` imports so they can be exercised under `node --test`.
 */

/**
 * Known debug CodeLens command ids.
 *
 * `executeCodeLensProvider` returns lenses from *every* provider registered for
 * a document, so selecting one by a fuzzy title substring meant Cmd+R could
 * invoke an arbitrary command contributed by another installed extension whose
 * lens title merely contained "debug". Matching on the id keeps the handoff to
 * the providers actually intended.
 */
export const DEBUG_COMMAND_IDS = new Set([
  "rust-analyzer.debugSingle",
  "rust-analyzer.debug",
]);

export function isDebugCommand(command: string | undefined): boolean {
  return command !== undefined && DEBUG_COMMAND_IDS.has(command);
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

/** Heuristic: does this resolved command line stay up rather than exiting? */
export function isLongRunning(shell: string): boolean {
  const lower = shell.toLowerCase();
  return LONG_RUNNING.some((p) => lower.includes(p));
}
