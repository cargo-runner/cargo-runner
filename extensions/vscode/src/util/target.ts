/**
 * Host → release-asset mapping and PATH-lookup helper resolution.
 *
 * Free of `vscode` imports so it can be exercised under `node --test`.
 */
import * as path from "node:path";

/** Map host platform/arch to the rustc target triple used in release assets. */
export function rustcTarget(
  platform: NodeJS.Platform = process.platform,
  arch: string = process.arch,
): string {
  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (platform === "linux" && arch === "arm64") {
    return "aarch64-unknown-linux-gnu";
  }
  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (platform === "win32" && arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

/**
 * Path to the PATH-lookup helper.
 *
 * On Windows this must be absolute: both libuv's resolution of a bare program
 * name and `where.exe` itself consult the current directory before PATH, so a
 * repository shipping `where.exe` would otherwise supply the helper.
 *
 * On POSIX it stays a bare name deliberately — PATH resolution there never
 * consults the current directory, and `which` is not at a fixed location
 * across distributions.
 */
export function lookupHelper(
  platform: NodeJS.Platform = process.platform,
  systemRoot: string = process.env.SystemRoot || "C:\\Windows",
): string {
  if (platform !== "win32") {
    return "which";
  }
  return path.win32.join(systemRoot, "System32", "where.exe");
}
