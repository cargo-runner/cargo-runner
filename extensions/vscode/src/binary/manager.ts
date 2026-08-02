import * as vscode from "vscode";
import * as path from "node:path";
import * as fs from "node:fs";
import * as https from "node:https";
import * as crypto from "node:crypto";
import * as os from "node:os";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as tar from "tar";
import { digestForAsset } from "../util/checksum";
import { isValidRepoSlug } from "../util/repo";
import { isNewerSemver, tagToSemver } from "../util/semver";
import { lookupHelper, rustcTarget as resolveRustcTarget } from "../util/target";

const execFileAsync = promisify(execFile);

const DEFAULT_RELEASE_REPO = "cargo-runner/cargo-runner";

export { isValidRepoSlug, isNewerSemver, tagToSemver };

/** Thrown when the CLI is missing and the user dismissed / timed out the prompt. */
export class CliMissingError extends Error {
  constructor(message = "Cargo Runner CLI is not installed") {
    super(message);
    this.name = "CliMissingError";
  }
}

/**
 * Resolves and downloads the cargo-runner CLI binary.
 *
 * Version parity: downloads `cargo-runner-cli-v{extensionVersion}` so the CLI
 * matches the VS Code extension version (e.g. both 1.6.2).
 *
 * Asset pattern (from release workflow):
 *   cargo-runner-cli-{rustc-target}-v{version}.tar.gz
 * containing `cargo-runner` or `cargo-runner.exe`.
 */
export class BinaryManager {
  private readonly binaryDirName = "bin";
  private downloadInFlight: Promise<string> | null = null;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly output: vscode.OutputChannel,
  ) {}

  private config() {
    return vscode.workspace.getConfiguration("cargoRunner");
  }

  private releaseRepo(): string {
    const configured = (this.config().get<string>("releaseRepo") || "").trim();
    if (!configured) {
      return DEFAULT_RELEASE_REPO;
    }
    // This value is interpolated into release URLs. Without a shape check,
    // `..` segments or an embedded scheme could retarget the download away
    // from the intended GitHub repo. (URL normalization means `..` could only
    // ever reach another path on the same host, but there is no reason to
    // accept it.)
    if (!isValidRepoSlug(configured)) {
      this.output.appendLine(
        `Ignoring invalid cargoRunner.releaseRepo "${configured}" (expected "owner/repo"); using ${DEFAULT_RELEASE_REPO}`,
      );
      return DEFAULT_RELEASE_REPO;
    }
    return configured;
  }

  /** Extension package version — kept in lockstep with CLI releases. */
  extensionVersion(): string {
    return String(this.context.extension.packageJSON.version || "1.6.2");
  }

  /** Release tag for this extension version. */
  expectedReleaseTag(): string {
    return `cargo-runner-cli-v${this.extensionVersion()}`;
  }

  /** Map host platform/arch to rustc target triple used in release assets. */
  rustcTarget(): string {
    return resolveRustcTarget();
  }

  binaryFileName(): string {
    return process.platform === "win32" ? "cargo-runner.exe" : "cargo-runner";
  }

  managedBinaryPath(): string {
    return path.join(
      this.context.globalStorageUri.fsPath,
      this.binaryDirName,
      this.binaryFileName(),
    );
  }

  /**
   * True when a runnable binary is already available (custom path, PATH, or managed).
   * Does not download.
   */
  async isCliAvailable(): Promise<boolean> {
    try {
      const resolved = await this.resolveExisting();
      return resolved !== null;
    } catch {
      return false;
    }
  }

  /**
   * Resolve an existing binary without downloading.
   * Returns absolute path or `cargo-runner` if on PATH.
   */
  async resolveExisting(): Promise<string | null> {
    const custom = (this.config().get<string>("path") || "").trim();
    if (custom) {
      if (custom === "cargo-runner" || custom === "cargo-runner.exe") {
        const onPath = await this.findOnPath();
        return onPath;
      }
      if (fs.existsSync(custom) && (await this.verifyExecutable(custom))) {
        return custom;
      }
      return null;
    }

    const onPath = await this.findOnPath();
    if (onPath) {
      return onPath;
    }

    const managed = this.managedBinaryPath();
    if (fs.existsSync(managed)) {
      await this.makeExecutable(managed);
      if (await this.verifyExecutable(managed)) {
        return managed;
      }
      this.output.appendLine(
        `Managed binary exists but is not executable: ${managed}`,
      );
    }

    return null;
  }

  /**
   * Ensure CLI is available. If missing, show a short-lived prompt with Download.
   * Does **not** silently download — user must confirm via the toast action.
   */
  async ensureBinary(): Promise<string> {
    const existing = await this.resolveExisting();
    if (existing) {
      this.output.appendLine(`Using cargo-runner: ${existing}`);
      return existing;
    }
    const installed = await this.promptMissingCli("run");
    if (!installed) {
      throw new CliMissingError(
        "Cargo Runner CLI is not installed. Use “Download CLI” or Cargo Runner: Setup Binary.",
      );
    }
    return installed;
  }

  /**
   * First-run / activation prompt: offer to install the CLI matching this extension.
   * If CLI is already present, checks GitHub for a newer CLI release.
   */
  async promptInstallOnActivate(): Promise<void> {
    if (!(await this.isCliAvailable())) {
      const ver = this.extensionVersion();
      const choice = await vscode.window.showInformationMessage(
        `Cargo Runner needs the CLI (v${ver}) to run, test, and debug. Download it now?`,
        "Download CLI",
        "Later",
      );

      if (choice === "Download CLI") {
        try {
          await this.downloadAndInstall();
          vscode.window.showInformationMessage(
            `Cargo Runner CLI v${ver} installed and ready.`,
          );
        } catch (e) {
          vscode.window.showErrorMessage(
            `Failed to download Cargo Runner CLI: ${e}. Try: cargo binstall cargo-runner-cli`,
          );
        }
      } else {
        vscode.window.setStatusBarMessage(
          `$(warning) Cargo Runner: CLI not installed — Cmd+R will prompt to download`,
          5000,
        );
      }
      return;
    }

    // CLI present — offer upgrade when a newer GitHub release exists
    await this.checkForCliUpdates();
  }

  /**
   * Compare installed CLI version to the latest GitHub release.
   * Prompts even when the VS Code extension itself has not been updated.
   *
   * Throttled (default once per 24h) and respects “Skip this version”.
   */
  async checkForCliUpdates(options?: { force?: boolean }): Promise<void> {
    if (!this.config().get<boolean>("checkCliUpdates", true) && !options?.force) {
      return;
    }

    try {
      const binary = await this.resolveExisting();
      if (!binary) {
        return;
      }

      const installed = await this.getInstalledCliVersion(binary);
      if (!installed) {
        this.output.appendLine("Could not parse installed CLI version");
        return;
      }

      const latestTag = await this.getLatestVersion();
      const latest = tagToSemver(latestTag);
      if (!latest) {
        return;
      }

      if (!isNewerSemver(latest, installed)) {
        this.output.appendLine(
          `CLI up to date: installed v${installed}, latest v${latest}`,
        );
        return;
      }

      const skipped = this.context.globalState.get<string>(
        "cargoRunner.skipCliVersion",
      );
      if (skipped === latest && !options?.force) {
        this.output.appendLine(`User skipped CLI update to v${latest}`);
        return;
      }

      if (!options?.force) {
        const lastPrompt = this.context.globalState.get<number>(
          "cargoRunner.lastCliUpdatePromptAt",
          0,
        );
        const hours = this.config().get<number>("cliUpdateCheckIntervalHours", 24);
        const intervalMs = Math.max(1, hours) * 60 * 60 * 1000;
        if (Date.now() - lastPrompt < intervalMs) {
          this.output.appendLine(
            `CLI update v${latest} available; prompt throttled (every ${hours}h)`,
          );
          return;
        }
      }

      await this.context.globalState.update(
        "cargoRunner.lastCliUpdatePromptAt",
        Date.now(),
      );

      const extVer = this.extensionVersion();
      const msg =
        `Cargo Runner CLI v${latest} is available (you have v${installed}). ` +
        `Extension is v${extVer} — you can use a newer CLI without updating the extension.`;

      vscode.window.setStatusBarMessage(
        `$(cloud-download) Cargo Runner CLI v${latest} available`,
        5000,
      );

      const choice = await vscode.window.showInformationMessage(
        msg,
        "Download Update",
        "Later",
        "Skip this version",
      );

      if (choice === "Download Update") {
        try {
          await this.downloadAndInstall(latestTag);
          // Clear skip for this version after successful install
          const skip = this.context.globalState.get<string>(
            "cargoRunner.skipCliVersion",
          );
          if (skip === latest) {
            await this.context.globalState.update(
              "cargoRunner.skipCliVersion",
              undefined,
            );
          }
          vscode.window.showInformationMessage(
            `Cargo Runner CLI updated to v${latest}.`,
          );
        } catch (e) {
          vscode.window.showErrorMessage(`CLI update failed: ${e}`);
        }
      } else if (choice === "Skip this version") {
        await this.context.globalState.update(
          "cargoRunner.skipCliVersion",
          latest,
        );
        vscode.window.setStatusBarMessage(
          `$(check) Won't remind about CLI v${latest}`,
          5000,
        );
      }
    } catch (e) {
      this.output.appendLine(`CLI update check failed: ${e}`);
    }
  }

  /**
   * Parse semver from `cargo-runner --version` (e.g. "cargo-runner 1.6.2").
   */
  async getInstalledCliVersion(binaryPath?: string): Promise<string | null> {
    const bin = binaryPath ?? (await this.resolveExisting());
    if (!bin) {
      return null;
    }
    try {
      const { stdout } = await execFileAsync(bin, ["--version"], {
        timeout: 10_000,
      });
      const match = stdout.match(/(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)/);
      return match ? match[1] : null;
    } catch {
      return null;
    }
  }

  /**
   * Cmd+R / Cmd+Shift+R missing-CLI UX:
   * - Status bar toast auto-clears after 5s
   * - Error notification with “Download CLI”; waiting races 5s so we don't block forever
   * - Clicking Download installs the binary (chmod +x + verify)
   */
  async promptMissingCli(
    reason: "run" | "activate" = "run",
  ): Promise<string | null> {
    if (this.downloadInFlight) {
      return this.downloadInFlight;
    }

    const ver = this.extensionVersion();
    const msg =
      reason === "activate"
        ? `Cargo Runner CLI (v${ver}) is required. Download now?`
        : `Cargo Runner CLI (v${ver}) is not installed. Click Download CLI to install.`;

    // Auto-clearing status bar toast (5 seconds)
    vscode.window.setStatusBarMessage(
      `$(error) ${msg}`,
      5000,
    );

    const choice = await Promise.race([
      vscode.window.showErrorMessage(msg, "Download CLI", "Dismiss"),
      delay(5000).then(() => undefined as string | undefined),
    ]);

    if (choice === "Download CLI") {
      try {
        return await this.downloadAndInstall();
      } catch (e) {
        vscode.window.showErrorMessage(
          `Download failed: ${e}. Try: cargo binstall cargo-runner-cli`,
        );
        return null;
      }
    }

    return null;
  }

  /**
   * Download CLI, extract, chmod +x, verify.
   * @param preferredTag optional GitHub tag (e.g. cargo-runner-cli-v1.7.0).
   *   When omitted: try extension-version tag, then latest.
   */
  async downloadAndInstall(preferredTag?: string): Promise<string> {
    if (this.downloadInFlight) {
      return this.downloadInFlight;
    }

    this.downloadInFlight = this._downloadAndInstall(preferredTag).finally(
      () => {
        this.downloadInFlight = null;
      },
    );
    return this.downloadInFlight;
  }

  private async _downloadAndInstall(preferredTag?: string): Promise<string> {
    const managed = this.managedBinaryPath();

    return vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "Cargo Runner",
        cancellable: false,
      },
      async (progress) => {
        const label = preferredTag
          ? tagToSemver(preferredTag) || preferredTag
          : this.extensionVersion();
        progress.report({ message: `Downloading CLI v${label}…` });

        // Remove previous managed binary so we don't leave a stale file
        if (fs.existsSync(managed)) {
          try {
            await fs.promises.unlink(managed);
          } catch {
            // ignore
          }
        }

        let tag = preferredTag || this.expectedReleaseTag();
        try {
          await this.downloadBinary(tag);
        } catch (firstErr) {
          if (preferredTag) {
            // Caller asked for a specific release — don't silently downgrade
            throw firstErr;
          }
          this.output.appendLine(
            `Download for ${tag} failed (${firstErr}); trying latest release…`,
          );
          progress.report({ message: "Trying latest GitHub release…" });
          tag = await this.getLatestVersion();
          await this.downloadBinary(tag);
          vscode.window.showWarningMessage(
            `Installed CLI ${tag} (extension is v${this.extensionVersion()}). Prefer releasing matching tags for full parity.`,
          );
        }

        await this.makeExecutable(managed);
        if (!(await this.verifyExecutable(managed))) {
          throw new Error(
            `Downloaded binary is not executable or failed --version: ${managed}`,
          );
        }

        // Prefer managed path over PATH after install/update
        await this.config().update(
          "path",
          managed,
          vscode.ConfigurationTarget.Global,
        );

        progress.report({ message: "CLI ready" });
        this.output.appendLine(`CLI installed: ${managed}`);
        return managed;
      },
    );
  }

  async updateBinary(): Promise<string> {
    const managed = this.managedBinaryPath();
    if (fs.existsSync(managed)) {
      await fs.promises.unlink(managed);
    }
    const versionFile = path.join(path.dirname(managed), "version.txt");
    if (fs.existsSync(versionFile)) {
      await fs.promises.unlink(versionFile);
    }
    // Clear custom path so we re-download managed
    const custom = (this.config().get<string>("path") || "").trim();
    if (custom === managed) {
      await this.config().update("path", "", vscode.ConfigurationTarget.Global);
    }
    // Prefer newest GitHub release when explicitly updating
    try {
      const latest = await this.getLatestVersion();
      return await this.downloadAndInstall(latest);
    } catch {
      return this.downloadAndInstall();
    }
  }

  private async findOnPath(): Promise<string | null> {
    try {
      const { stdout } = await execFileAsync(
        lookupHelper(),
        ["cargo-runner"],
        // Run from the user's home, never the workspace. Two things search the
        // current directory before PATH on Windows: libuv's own program
        // resolution for a bare name, and `where.exe` itself. Without this, a
        // repository containing `where.exe` — or a `cargo-runner.exe` that
        // `where` would report first — gets executed by verifyExecutable below,
        // during activation.
        { cwd: os.homedir() },
      );
      const first = stdout
        .split(/\r?\n/)
        .map((s) => s.trim())
        .find(Boolean);
      if (!first) {
        return null;
      }
      if (!(await this.verifyExecutable(first))) {
        return null;
      }
      this.output.appendLine(`Found cargo-runner on PATH: ${first}`);
      return first;
    } catch {
      return null;
    }
  }

  /** chmod +x and strip macOS quarantine when possible. */
  async makeExecutable(binaryPath: string): Promise<void> {
    if (process.platform === "win32") {
      return;
    }
    try {
      await fs.promises.chmod(binaryPath, 0o755);
    } catch (e) {
      this.output.appendLine(`chmod failed: ${e}`);
    }
    if (process.platform === "darwin") {
      try {
        // Absolute path, and not from the workspace directory — same reasoning
        // as findOnPath.
        await execFileAsync(
          "/usr/bin/xattr",
          ["-d", "com.apple.quarantine", binaryPath],
          { cwd: os.homedir() },
        );
      } catch {
        // attribute may not exist — fine
      }
    }
  }

  /** Confirm the binary runs (`--version` exits 0). */
  async verifyExecutable(binaryPath: string): Promise<boolean> {
    try {
      if (!fs.existsSync(binaryPath) && binaryPath !== "cargo-runner") {
        // still try PATH name
      }
      const { stdout } = await execFileAsync(binaryPath, ["--version"], {
        timeout: 10_000,
      });
      this.output.appendLine(`Verified: ${stdout.trim()}`);
      return true;
    } catch (e) {
      this.output.appendLine(`verifyExecutable failed for ${binaryPath}: ${e}`);
      return false;
    }
  }

  async getLatestVersion(): Promise<string> {
    const repo = this.releaseRepo();
    const url = `https://api.github.com/repos/${repo}/releases/latest`;
    const body = await this.httpsGet(url, {
      "User-Agent": "cargo-runner-vscode",
      Accept: "application/vnd.github+json",
    });
    const release = JSON.parse(body) as { tag_name?: unknown };
    // The `as` cast is not a runtime check, and this value becomes both a URL
    // segment and a local path component.
    if (typeof release.tag_name !== "string" || !release.tag_name) {
      throw new Error("GitHub latest release has no usable tag_name");
    }
    if (!/^[A-Za-z0-9._-]+$/.test(release.tag_name)) {
      throw new Error(`GitHub returned a malformed tag_name: ${release.tag_name}`);
    }
    return release.tag_name;
  }

  private async downloadBinary(tag: string): Promise<void> {
    const target = this.rustcTarget();
    // `tag` may come from the GitHub API (getLatestVersion), and it is used
    // both in the URL and as a local path component below — so it has to be a
    // plain name, not something containing separators or `..`.
    if (!/^[A-Za-z0-9._-]+$/.test(tag) || tag === "." || tag === "..") {
      throw new Error(`Refusing to use malformed release tag: ${tag}`);
    }
    // tag: cargo-runner-cli-v1.6.2 → version 1.6.2
    const version = tag.replace(/^cargo-runner-cli-v/, "").replace(/^v/, "");
    const asset = `cargo-runner-cli-${target}-v${version}.tar.gz`;
    const repo = this.releaseRepo();
    const base = `https://github.com/${repo}/releases/download/${tag}`;
    const url = `${base}/${asset}`;

    this.output.appendLine(`Downloading: ${url}`);

    const destDir = path.dirname(this.managedBinaryPath());
    await fs.promises.mkdir(destDir, { recursive: true });

    const tmpArchive = path.join(destDir, asset);
    await this.downloadFile(url, tmpArchive);

    // Verify before extracting. This archive is about to be unpacked, marked
    // executable and run (and on macOS have its quarantine bit stripped), so
    // HTTPS alone is not enough — it authenticates the host, not the bytes.
    await this.verifyChecksum(tmpArchive, asset, `${base}/SHA256SUMS`);

    // Extract: archive contains a folder with cargo-runner inside.
    await tar.x({
      file: tmpArchive,
      cwd: destDir,
      strip: 1,
      strict: true,
      // Only the binary is wanted; ignore anything else the archive carries so
      // a tampered tarball cannot litter or clobber the managed bin directory.
      filter: (p: string) =>
        path.basename(p) === this.binaryFileName() ||
        path.basename(p) === "README.md" ||
        path.basename(p) === "LICENSE",
    });

    const binaryPath = this.managedBinaryPath();
    if (!fs.existsSync(binaryPath)) {
      const found = await this.findExtractedBinary(destDir);
      if (found && found !== binaryPath) {
        await fs.promises.rename(found, binaryPath);
      }
    }

    if (!fs.existsSync(binaryPath)) {
      throw new Error(`Binary not found after extract: ${binaryPath}`);
    }

    await this.makeExecutable(binaryPath);

    await fs.promises.writeFile(path.join(destDir, "version.txt"), tag, "utf8");
    try {
      await fs.promises.unlink(tmpArchive);
    } catch {
      // ignore
    }

    this.output.appendLine(`Installed cargo-runner ${tag} → ${binaryPath}`);
  }

  /**
   * Verify a downloaded archive against the release's SHA256SUMS manifest.
   *
   * Fails closed: a missing manifest, a missing entry for this asset, or a
   * mismatch all abort the install and delete the archive. Releases published
   * before SHA256SUMS existed will fail here — that is intentional; the fix is
   * to re-publish with the manifest rather than to skip verification.
   */
  private async verifyChecksum(
    archivePath: string,
    assetName: string,
    sumsUrl: string,
  ): Promise<void> {
    const discard = async () => {
      try {
        await fs.promises.unlink(archivePath);
      } catch {
        // best effort
      }
    };

    let manifest: string;
    try {
      manifest = await this.httpsGet(sumsUrl, {
        "User-Agent": "cargo-runner-vscode",
      });
    } catch (e) {
      await discard();
      throw new Error(
        `Could not fetch ${sumsUrl} to verify the download (${e}). ` +
          `Refusing to install an unverified binary.`,
      );
    }

    const expected = digestForAsset(manifest, assetName);
    if (!expected) {
      await discard();
      throw new Error(
        `No SHA256SUMS entry for ${assetName}. Refusing to install an unverified binary.`,
      );
    }

    const actual = await new Promise<string>((resolve, reject) => {
      const hash = crypto.createHash("sha256");
      const stream = fs.createReadStream(archivePath);
      stream.on("error", reject);
      stream.on("data", (chunk) => hash.update(chunk));
      stream.on("end", () => resolve(hash.digest("hex")));
    });

    if (actual !== expected) {
      await discard();
      throw new Error(
        `Checksum mismatch for ${assetName}: expected ${expected}, got ${actual}. ` +
          `The download was discarded.`,
      );
    }

    this.output.appendLine(`Verified sha256 ${actual} for ${assetName}`);
  }

  private async findExtractedBinary(dir: string): Promise<string | null> {
    const name = this.binaryFileName();
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });
    for (const e of entries) {
      const full = path.join(dir, e.name);
      if (e.isFile() && e.name === name) {
        return full;
      }
      if (e.isDirectory() && !e.name.startsWith(".")) {
        const nested = await this.findExtractedBinary(full);
        if (nested) {
          return nested;
        }
      }
    }
    return null;
  }

  private downloadFile(url: string, dest: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const follow = (u: string, redirects = 0) => {
        if (redirects > 5) {
          reject(new Error("Too many redirects"));
          return;
        }
        if (!u.startsWith("https://")) {
          reject(new Error(`Refusing non-HTTPS download URL: ${u}`));
          return;
        }
        https
          .get(u, { headers: { "User-Agent": "cargo-runner-vscode" } }, (res) => {
            if (
              res.statusCode &&
              res.statusCode >= 300 &&
              res.statusCode < 400 &&
              res.headers.location
            ) {
              // Resolve relative redirects against the current URL, then
              // re-check the scheme so a redirect cannot downgrade to plain
              // HTTP (the binary is chmod +x'd and executed after download).
              res.resume();
              follow(new URL(res.headers.location, u).toString(), redirects + 1);
              return;
            }
            if (res.statusCode !== 200) {
              reject(new Error(`Download failed: HTTP ${res.statusCode} for ${u}`));
              return;
            }
            const stream = fs.createWriteStream(dest, { mode: 0o644 });
            res.pipe(stream);
            stream.on("finish", () => {
              stream.close();
              resolve();
            });
            stream.on("error", reject);
          })
          .on("error", reject);
      };
      follow(url);
    });
  }

  /**
   * GET a small text body over HTTPS.
   *
   * Bounded on purpose: this runs on every activation (update check) and now
   * also fetches SHA256SUMS, so it must not be able to buffer without limit or
   * hang the promise forever.
   */
  private httpsGet(
    url: string,
    headers: Record<string, string>,
    redirects = 0,
  ): Promise<string> {
    const MAX_BYTES = 1024 * 1024;
    const TIMEOUT_MS = 30_000;

    return new Promise((resolve, reject) => {
      if (!url.startsWith("https://")) {
        reject(new Error(`Refusing non-HTTPS URL: ${url}`));
        return;
      }
      if (redirects > 5) {
        reject(new Error("Too many redirects"));
        return;
      }
      const req = https.get(url, { headers }, (res) => {
        // Release assets redirect to objects.githubusercontent.com.
        if (
          res.statusCode &&
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          res.resume();
          this.httpsGet(
            new URL(res.headers.location, url).toString(),
            headers,
            redirects + 1,
          ).then(resolve, reject);
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`HTTP ${res.statusCode} for ${url}`));
          return;
        }
        let data = "";
        let size = 0;
        res.on("data", (c: Buffer) => {
          size += c.length;
          if (size > MAX_BYTES) {
            res.destroy();
            reject(new Error(`Response from ${url} exceeded ${MAX_BYTES} bytes`));
            return;
          }
          data += c;
        });
        res.on("error", reject);
        res.on("end", () => resolve(data));
      });
      req.setTimeout(TIMEOUT_MS, () => {
        req.destroy(new Error(`Timed out after ${TIMEOUT_MS}ms for ${url}`));
      });
      req.on("error", reject);
    });
  }
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

