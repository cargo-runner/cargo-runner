# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-08-15

### Added
- Unified runner architecture with `UnifiedRunner` as the main entry point
- Comprehensive Bazel target detection from BUILD files
- Support for `rust_test` targets that reference binaries via `crate` attribute
- Automatic optimization flags (`-c opt`) for benchmark binaries
- `--nocapture` flag by default for all test commands to show println! output
- Comprehensive test coverage for all new features
- Detection of `cargo_build_script` targets for build.rs files
- Working directory auto-detection for Bazel workspaces

### Changed
- Renamed `runner_v2` module to `runners` for better clarity
- Module path resolution now correctly excludes `bin::` prefix for files in `src/bin/`
- Benchmark files in `benches/` directory run as binaries (not tests) at file level
- Improved separation of concerns between build system detection and command execution

### Fixed
- Fixed Bazel target detection for test targets referencing binaries
- Fixed incorrect module paths for files in `src/bin/` directory
- Fixed benchmark file execution to run binaries instead of tests
- Fixed "not invoked from within a workspace" errors by setting correct working directory
- Fixed module name filtering when module paths are empty

### Technical Details
- Bazel commands now use `--test_arg` prefix for each test argument
- Build system detection follows priority: Bazel > Cargo > Rustc
- Tree-sitter based AST parsing provides accurate runnable detection without compilation

## [0.0.1] - Previous Version

### Initial Features
- Basic cargo command generation for tests and binaries
- Simple pattern detection for test functions
- Basic support for cargo projects

---

Note: Version 0.0.1 represents the previous iteration before the major Bazel integration refactor.