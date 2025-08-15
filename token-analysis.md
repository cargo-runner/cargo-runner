# Cargo Runner Override Command Token Analysis

## Currently Used Tokens

### 1. **`@` - Command/Subcommand Token**
- **Usage**: `@command.subcommand`
- **Examples**: 
  - `@cargo.nextest` - Sets cargo with nextest subcommand
  - `@dx.serve` - Sets dx command with serve subcommand
  - `@trunk` - Sets trunk as the command
  - `@bazel.test` - Sets bazel with test subcommand
- **Behavior**: Everything after `@` until the next token is parsed as command.subcommand

### 2. **`+` - Channel Token**
- **Usage**: `+channel`
- **Examples**: 
  - `+nightly` - Sets Rust channel to nightly
  - `+stable` - Sets Rust channel to stable
  - `+beta` - Sets Rust channel to beta
- **Behavior**: The `+` prefix indicates a Rust toolchain channel

### 3. **`/` - Test Binary Args Separator**
- **Usage**: `/args` or `/ args`
- **Examples**: 
  - `/--nocapture` - Pass --nocapture to test binary
  - `/ --test-threads=1 --show-output` - Multiple test args
- **Behavior**: Acts like `--` in cargo test, everything after goes to the test binary

### 4. **`-` - Removal/Negation Token**
- **Usage**: `-field` or `-ENVVAR`
- **Examples**: 
  - `-command` - Remove command field
  - `-env` - Remove all environment variables
  - `-RUST_LOG` - Remove specific env var RUST_LOG
  - `-` (alone) - Remove entire override
- **Behavior**: Removes fields or specific values from configuration

### 5. **`SCREAMING_CASE=value` - Environment Variable**
- **Usage**: `ENV_VAR=value`
- **Examples**: 
  - `RUST_LOG=debug` - Set RUST_LOG env var
  - `DATABASE_URL=postgres://localhost` - Set DATABASE_URL
- **Behavior**: Detected by all-caps with underscores pattern containing `=`

### 6. **`--` - Regular Argument Prefix**
- **Usage**: `--arg`
- **Examples**: 
  - `--release` - Added to extra_args
  - `--verbose` - Added to extra_args
- **Behavior**: Standard command-line argument, goes to extra_args

## Available Tokens for Future Use

### Safe to Use:
1. **`%` - Perfect for framework type declaration**
   - `%test` - Configure test_framework
   - `%bin` or `%binary` - Configure binary_framework
   - `%bench` - Configure benchmark_framework
   - `%doc` - Configure doc_test_framework (Bazel only)
   - Clear, intuitive, no shell conflicts

2. **`:` - Could be used for features or other config**
   - `:all` - All features
   - `:feat1,feat2` - Specific features
   - `:package=name` - Set package name

3. **`!` - UNSAFE - Shell history expansion**
   - `!!` - Repeat last command
   - `!test` - Run last command starting with "test"
   - Would cause unexpected behavior
   - Should NOT be used

4. **`^` - Could be used for linked projects**
   - `^../other-project` - Add linked project

5. **`~` - Could be used for workspace operations**
   - `~workspace` - Set bazel workspace
   - `~target` - Set bazel target template

6. **`^` - Safe for linked projects**
   - `^../other-project` - Add linked project
   - No shell conflicts

7. **`~` - Safe for workspace/special configs**
   - `~workspace-name` - Set Bazel workspace
   - Only expands when followed by `/` or username

8. **`.` - Could be used for property access**
   - `.package=name` - Set package
   - But might be confusing with file extensions

9. **`=` - Already used in env vars**
   - Could extend for templates: `target={target}`

## Tokens to Avoid:

### Shell Special Characters (unsafe):
1. **`!` - Shell history expansion**
   - `!!` - Repeats last command
   - `!$` - Last argument of previous command
   - `!test` - Last command starting with "test"
   - Very dangerous for command parsing

2. **`#` - Shell comment character**
   - Everything after `#` is treated as comment
   - `cargo runner override file.rs #all --release` would ignore `--release`
   - Cannot be used safely in command line

3. **`$` - Shell variable expansion**
   - Would be interpreted by shell
   - Example: `$HOME` would expand

4. **`|` - Shell pipe**
   - Would break command parsing
   - Already used in rustc pipe config

3. **`>`, `<` - Shell redirection**
   - Would redirect input/output

4. **`;` - Shell command separator**
   - Would execute multiple commands

5. **`&` - Shell background operator**
   - Might run in background (on some shells)

6. **`` ` `` - Shell command substitution**
   - Would execute embedded commands

7. **`(`, `)` - Shell subshell or function**
   - Could create subshells

8. **`{`, `}` - Shell brace expansion**
   - Might expand to multiple args

9. **`[`, `]` - Shell test command**
   - Might be interpreted as test

10. **`\` - Shell escape character**
    - Would affect parsing

11. **`"`, `'` - Shell quoting**
    - Already used for quoting args

12. **Space** - Argument separator
    - Already used to separate arguments

## Proposed Extended Token System

Based on the analysis, here's a proposed extended syntax:

```bash
# Framework type selection (using %)
cargo runner override src/lib.rs:10 %test @cargo.nextest    # Configure test_framework
cargo runner override src/lib.rs:10 %bin --release          # Configure binary_framework
cargo runner override src/lib.rs:10 %bench                  # Configure benchmark_framework

# Features (using :)
cargo runner override src/lib.rs:10 :all                    # --all-features
cargo runner override src/lib.rs:10 :web,desktop            # --features=web,desktop

# Package selection
cargo runner override src/lib.rs:10 :package=my-package     # Set package

# Linked projects
cargo runner override src/lib.rs:10 ^../shared ^../common   # Multiple linked projects

# Bazel workspace/targets
cargo runner override src/lib.rs:10 ~my_workspace     # Set workspace
cargo runner override src/lib.rs:10 target=//:test    # Set target template

# Rustc phases
cargo runner override src/lib.rs:10 :build -C opt-level=3   # Build phase args
cargo runner override src/lib.rs:10 :exec RUST_LOG=debug    # Exec phase env

# Combined example
cargo runner override src/lib.rs:10 %test @cargo.nextest +nightly :all --release RUST_LOG=debug /--nocapture
```

## Implementation Priority

1. **High Priority** (most useful):
   - `%` for framework type selection (%test, %bin, %bench)
   - `:` for features (:all, :feat1,feat2)
   - `:` for config properties (:package=name)

2. **Medium Priority**:
   - `^` for linked projects
   - `:` for rustc phases
   - `~` for bazel workspace

3. **Low Priority**:
   - Extended `=` usage for templates
   - Other tokens as needed

## Notes

- The current token system is well-designed to avoid shell conflicts
- Most dangerous shell characters are avoided
- The existing tokens (`@`, `+`, `/`, `-`) are intuitive
- New tokens should maintain this intuitive nature
- Consider backwards compatibility when adding new tokens