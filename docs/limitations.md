# Limitations & intentional hold-offs

Cargo Runner aims for fast, source-level resolution without driving a full
rustc/rustdoc expansion pipeline. These cases are **out of scope** (or limited)
by design.

| Case | Behavior | Escape hatch |
|------|----------|--------------|
| Macro / `#[doc = include_str!(…)]` / proc-macro doctests | Not detected for scoped run | Crate-level `cargo test --doc` or Bazel `rust_doc_test` |
| Indented (no-fence) rustdoc examples | Not detected | Use markdown fences `` ``` `` |
| Bazel per-example doctest | Runs **all** crate doctests | Accept crate-level run; note in dry-run `warnings` |
| ibazel | Not integrated | `cargo runner watch` (notify) or install `cargo-watch` |
| `watch` without a file:line | Project-level build/run/test only | `watch path/to/file.rs:LINE` replays resolved `run` command |

## Scoped doctests (supported)

Fenced examples in `///`, `//!`, or `/** */` attached to fn / struct / enum /
mod / union / **trait** / impl method. Fence tags `ignore`, `no_run`, and
`compile_fail` do not produce a “Run” action.

## IDE JSON errors

When using `--json` (or `run --dry-run --json`), failures emit a structured
object on stdout (see `docs/ide-protocol.md`).
