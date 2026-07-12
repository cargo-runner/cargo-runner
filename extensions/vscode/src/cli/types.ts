/** Types mirrored from docs/ide-protocol.md (protocol_version 1). */

export interface ScopePosition {
  line: number;
  character?: number;
}

export interface Scope {
  start: ScopePosition;
  end: ScopePosition;
}

export type RunnableKind =
  | { Test: { test_name: string; is_async: boolean } }
  | { DocTest: { struct_or_module_name: string; method_name?: string | null } }
  | { Benchmark: { bench_name: string } }
  | { Binary: { bin_name?: string | null } }
  | { ModuleTests: { module_name: string } }
  | { Standalone: { has_tests: boolean } }
  | { SingleFileScript: { shebang: string } };

export interface Runnable {
  label: string;
  scope: Scope;
  kind: RunnableKind;
  module_path: string;
  file_path: string;
}

export interface CommandPreview {
  program: string;
  args: string[];
  cwd?: string | null;
  shell: string;
}

export interface RunnableEntry extends Runnable {
  command?: CommandPreview | null;
}

export interface DryRunOutput {
  protocol_version: number;
  program: string;
  args: string[];
  cwd?: string | null;
  env: Record<string, string>;
  shell: string;
  strategy: string;
  runnable?: Runnable | null;
}

export interface RunnerContext {
  context_version: number;
  cwd: string;
  project_root?: string | null;
  file_path?: string | null;
  line?: number | null;
  build_system: string;
  file_kind: string;
  runnable_kind?: string | null;
  package_name?: string | null;
  recommended_target?: string | null;
}

export interface OverrideListEntry {
  config_path: string;
  override: {
    match?: {
      file_path?: string;
      function_name?: string;
      module_path?: string;
      package?: string;
    };
    cargo?: Record<string, unknown>;
    bazel?: Record<string, unknown>;
    rustc?: Record<string, unknown>;
  };
}
