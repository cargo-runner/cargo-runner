import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";
import * as vscode from "vscode";
import type {
  DryRunOutput,
  OverrideListEntry,
  RunnableEntry,
  RunnerContext,
} from "./types";

const execFileAsync = promisify(execFile);

export class CliClient {
  constructor(
    private readonly getBinary: () => Promise<string>,
    private readonly output: vscode.OutputChannel,
  ) {}

  private async bin(): Promise<string> {
    return this.getBinary();
  }

  /**
   * cargo-runner is installed as a cargo subcommand binary named `cargo-runner`,
   * which expects either `cargo runner …` invocation or direct argv as if it were
   * the `runner` subcommand of cargo. Our binary's main accepts `runner` args
   * when invoked as `cargo-runner <subcommand>`.
   *
   * We invoke: cargo-runner <args…>
   * matching `cargo runner <args…>`.
   */
  async runJson<T>(
    args: string[],
    options?: { cwd?: string; timeoutMs?: number },
  ): Promise<T> {
    const binary = await this.bin();
    const cwd =
      options?.cwd ||
      vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ||
      process.cwd();
    const timeout = options?.timeoutMs ?? 60_000;

    this.output.appendLine(`$ ${binary} ${args.join(" ")}`);

    try {
      const { stdout, stderr } = await execFileAsync(binary, args, {
        cwd,
        timeout,
        maxBuffer: 20 * 1024 * 1024,
        env: { ...process.env },
      });
      if (stderr?.trim()) {
        this.output.appendLine(stderr.trim());
      }
      return JSON.parse(stdout) as T;
    } catch (err: unknown) {
      const e = err as {
        stdout?: string;
        stderr?: string;
        message?: string;
      };
      if (e.stderr) {
        this.output.appendLine(e.stderr);
      }
      if (e.stdout) {
        this.output.appendLine(e.stdout);
        // Prefer structured IDE error JSON when present
        try {
          const parsed = JSON.parse(e.stdout.trim()) as {
            error?: boolean;
            message?: string;
          };
          if (parsed?.error && parsed.message) {
            throw new Error(parsed.message);
          }
        } catch (inner) {
          if (inner instanceof Error && inner.message && !inner.message.includes("JSON")) {
            throw inner;
          }
        }
      }
      throw new Error(e.stderr || e.message || String(err));
    }
  }

  async runnables(
    filePath?: string,
    opts?: { withCommands?: boolean; cwd?: string },
  ): Promise<RunnableEntry[]> {
    const args = ["runnables", "--json"];
    if (opts?.withCommands) {
      args.push("--with-commands");
    }
    if (filePath) {
      args.push(filePath);
    }
    return this.runJson<RunnableEntry[]>(args, { cwd: opts?.cwd });
  }

  async dryRun(fileArg: string, cwd?: string): Promise<DryRunOutput> {
    return this.runJson<DryRunOutput>(
      ["run", fileArg, "--dry-run", "--json"],
      { cwd },
    );
  }

  async context(fileArg?: string, cwd?: string): Promise<RunnerContext> {
    const args = ["context", "--json"];
    if (fileArg) {
      args.push(fileArg);
    }
    return this.runJson<RunnerContext>(args, { cwd });
  }

  async listOverrides(file?: string, cwd?: string): Promise<OverrideListEntry[]> {
    const args = ["override", "--list", "--json"];
    if (file) {
      args.push("--file", file);
    }
    return this.runJson<OverrideListEntry[]>(args, { cwd });
  }

  async setOverride(
    fileArg: string,
    tokens: string[],
    cwd?: string,
  ): Promise<void> {
    const binary = await this.bin();
    const workDir =
      cwd ||
      vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ||
      process.cwd();
    const args = ["override", fileArg, "--", ...tokens];
    this.output.appendLine(`$ ${binary} ${args.join(" ")}`);
    await execFileAsync(binary, args, {
      cwd: workDir,
      timeout: 30_000,
      env: { ...process.env },
    });
  }

  /** Spawn cargo-runner for interactive/long-running execution (non-JSON). */
  spawn(
    args: string[],
    options: { cwd?: string; env?: Record<string, string> },
  ): ReturnType<typeof spawn> {
    // Fire-and-forget; caller must await ensureBinary first.
    return spawn("cargo-runner", args, {
      cwd: options.cwd,
      env: { ...process.env, ...options.env },
      shell: false,
    });
  }
}
