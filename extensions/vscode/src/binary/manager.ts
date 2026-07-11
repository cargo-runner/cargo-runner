import * as vscode from "vscode";
import * as path from "node:path";
import * as fs from "node:fs";
import * as https from "node:https";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as tar from "tar";

const execFileAsync = promisify(execFile);

/**
 * Resolves and downloads the cargo-runner CLI binary.
 *
 * Asset pattern (from release workflow):
 *   cargo-runner-cli-{rustc-target}-v{version}.tar.gz
 * containing `cargo-runner` or `cargo-runner.exe`.
 */
export class BinaryManager {
  private readonly binaryDirName = "bin";

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly output: vscode.OutputChannel,
  ) {}

  private config() {
    return vscode.workspace.getConfiguration("cargoRunner");
  }

  private releaseRepo(): string {
    return this.config().get<string>("releaseRepo") || "cargo-runner/cargo-runner";
  }

  /** Map host platform/arch to rustc target triple used in release assets. */
  rustcTarget(): string {
    const platform = process.platform;
    const arch = process.arch;

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
   * Ensure a usable cargo-runner binary exists and return its absolute path
   * (or bare command name when using PATH).
   */
  async ensureBinary(): Promise<string> {
    const custom = (this.config().get<string>("path") || "").trim();
    if (custom) {
      if (custom === "cargo-runner" || custom === "cargo-runner.exe") {
        return custom;
      }
      if (fs.existsSync(custom)) {
        return custom;
      }
      throw new Error(`Custom cargo-runner path not found: ${custom}`);
    }

    // Prefer system PATH when available and recent enough.
    const onPath = await this.findOnPath();
    if (onPath) {
      this.output.appendLine(`Using cargo-runner from PATH: ${onPath}`);
      return onPath;
    }

    const managed = this.managedBinaryPath();
    if (fs.existsSync(managed)) {
      this.output.appendLine(`Using managed cargo-runner: ${managed}`);
      return managed;
    }

    return vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "Cargo Runner",
        cancellable: false,
      },
      async (progress) => {
        progress.report({ message: "Downloading cargo-runner binary…" });
        const version = await this.getLatestVersion();
        await this.downloadBinary(version);
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
    return this.ensureBinary();
  }

  private async findOnPath(): Promise<string | null> {
    try {
      const { stdout } = await execFileAsync(
        process.platform === "win32" ? "where" : "which",
        ["cargo-runner"],
      );
      const first = stdout.split(/\r?\n/).map((s) => s.trim()).find(Boolean);
      if (!first) {
        return null;
      }
      // Quick version check
      try {
        const { stdout: ver } = await execFileAsync(first, ["--version"]);
        this.output.appendLine(`Found on PATH: ${ver.trim()}`);
      } catch {
        // still usable
      }
      return first;
    } catch {
      return null;
    }
  }

  async getLatestVersion(): Promise<string> {
    const repo = this.releaseRepo();
    const url = `https://api.github.com/repos/${repo}/releases/latest`;
    const body = await this.httpsGet(url, {
      "User-Agent": "cargo-runner-vscode",
      Accept: "application/vnd.github+json",
    });
    const release = JSON.parse(body) as { tag_name?: string };
    if (!release.tag_name) {
      throw new Error("GitHub latest release has no tag_name");
    }
    return release.tag_name;
  }

  private async downloadBinary(tag: string): Promise<void> {
    const target = this.rustcTarget();
    // tag: cargo-runner-cli-v1.0.0 → version 1.0.0
    const version = tag.replace(/^cargo-runner-cli-v/, "").replace(/^v/, "");
    const asset = `cargo-runner-cli-${target}-v${version}.tar.gz`;
    const repo = this.releaseRepo();
    const url = `https://github.com/${repo}/releases/download/${tag}/${asset}`;

    this.output.appendLine(`Downloading: ${url}`);

    const destDir = path.dirname(this.managedBinaryPath());
    await fs.promises.mkdir(destDir, { recursive: true });

    const tmpArchive = path.join(destDir, asset);
    await this.downloadFile(url, tmpArchive);

    // Extract: archive contains a folder with cargo-runner inside.
    await tar.x({
      file: tmpArchive,
      cwd: destDir,
      strip: 1,
    });

    const binaryPath = this.managedBinaryPath();
    if (!fs.existsSync(binaryPath)) {
      // Fallback: search extracted tree
      const found = await this.findExtractedBinary(destDir);
      if (found && found !== binaryPath) {
        await fs.promises.rename(found, binaryPath);
      }
    }

    if (!fs.existsSync(binaryPath)) {
      throw new Error(`Binary not found after extract: ${binaryPath}`);
    }

    if (process.platform !== "win32") {
      await fs.promises.chmod(binaryPath, 0o755);
    }

    await fs.promises.writeFile(path.join(destDir, "version.txt"), tag, "utf8");
    try {
      await fs.promises.unlink(tmpArchive);
    } catch {
      // ignore
    }

    this.output.appendLine(`Installed cargo-runner ${tag} → ${binaryPath}`);
  }

  private async findExtractedBinary(dir: string): Promise<string | null> {
    const name = this.binaryFileName();
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });
    for (const e of entries) {
      const full = path.join(dir, e.name);
      if (e.isFile() && e.name === name) {
        return full;
      }
      if (e.isDirectory()) {
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
        https
          .get(u, { headers: { "User-Agent": "cargo-runner-vscode" } }, (res) => {
            if (
              res.statusCode &&
              res.statusCode >= 300 &&
              res.statusCode < 400 &&
              res.headers.location
            ) {
              follow(res.headers.location, redirects + 1);
              return;
            }
            if (res.statusCode !== 200) {
              reject(new Error(`Download failed: HTTP ${res.statusCode}`));
              return;
            }
            const stream = fs.createWriteStream(dest);
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

  private httpsGet(url: string, headers: Record<string, string>): Promise<string> {
    return new Promise((resolve, reject) => {
      https
        .get(url, { headers }, (res) => {
          if (res.statusCode !== 200) {
            reject(new Error(`HTTP ${res.statusCode} for ${url}`));
            return;
          }
          let data = "";
          res.on("data", (c) => {
            data += c;
          });
          res.on("end", () => resolve(data));
        })
        .on("error", reject);
    });
  }
}
