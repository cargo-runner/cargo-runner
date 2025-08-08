# Cargo Runner - Unified CLI

A powerful command-line tool for detecting and running Rust code at specific locations in files. It can show all available runnables in a file or find the best runnable at a specific line number.

## Installation

```bash
cargo install --path . --bin cargo-runner
```

Or run directly from the project:

```bash
cargo run --bin cargo-runner -- <command>
```

## Usage

### Basic Commands

```bash
# Show all runnables in a file
cargo-runner show src/lib.rs

# Show the best runnable at a specific line
cargo-runner show src/lib.rs:42

# Using file:line syntax
cargo-runner show tests/integration_test.rs:155

# Legacy mode (backward compatible)
cargo-runner src/lib.rs
cargo-runner src/lib.rs 42
```

### Quick Mode (for scripting)

```bash
# Output only the command, no decorations
CARGO_RUNNER_QUICK=1 cargo-runner show src/lib.rs:42
```

This outputs just the cargo command, making it perfect for scripting:

```bash
# Execute the command directly
$(CARGO_RUNNER_QUICK=1 cargo-runner show src/lib.rs:42)

# Use in scripts
#!/bin/bash
cmd=$(CARGO_RUNNER_QUICK=1 cargo-runner show $1)
if [ $? -eq 0 ]; then
    echo "Running: $cmd"
    $cmd
else
    echo "No runnable found at $1"
fi
```

## Examples

### Finding a test at a specific line:

```bash
$ cargo-runner show examples/showcase.rs:55

🎯 Best runnable at line 55:

📦 Run test 'test_add'
📏 Scope: lines 53-57
🚀 Command: cargo test --package cargo-runner -- cargo-runner::test_add::test_add --exact
🏷️  Type: Test function 'test_add'
📁 Module path: cargo-runner::test_add
```

### Quick mode for scripting:

```bash
$ CARGO_RUNNER_QUICK=1 cargo-runner show examples/showcase.rs:55
cargo test --package cargo-runner -- cargo-runner::test_add::test_add --exact
```

### Showing all runnables:

```bash
$ cargo-runner show examples/showcase.rs

🔍 Scanning: examples/showcase.rs
================================================================================
✅ Found 11 runnable(s):

1. Run test 'test_add'
   📏 Scope: lines 53-57
   🚀 Command: cargo test --package cargo-runner -- cargo-runner::test_add --exact
   📦 Type: Test function 'test_add'
   📁 Module path: cargo-runner::test_add

2. Run benchmark 'bench_add'
   📏 Scope: lines 101-105
   🚀 Command: cargo bench --package cargo-runner
   📦 Type: Benchmark 'bench_add'
   📁 Module path: cargo-runner::bench_add

[... more runnables ...]
================================================================================
```

## Detectable Runnables

The tool can detect:
- ✅ Test functions (`#[test]`)
- ✅ Async tests (`#[tokio::test]`)
- ✅ Benchmarks (`#[bench]`)
- ✅ Binary/main functions
- ✅ Doc tests in `///` comments
- ✅ Test modules

## Integration Examples

### Vim/Neovim Integration

Add to your vim config:

```vim
" Run test at current line
nnoremap <leader>rt :execute '!' . system('CARGO_RUNNER_QUICK=1 cargo-runner show ' . expand('%') . ':' . line('.'))<CR>

" Function to run test at cursor
function! RunTestAtCursor()
    let cmd = system('CARGO_RUNNER_QUICK=1 cargo-runner show ' . expand('%') . ':' . line('.'))
    if v:shell_error == 0
        execute '!' . cmd
    else
        echo "No runnable found at current line"
    endif
endfunction
nnoremap <leader>rc :call RunTestAtCursor()<CR>
```

### VS Code Task

```json
{
    "label": "Run Test at Cursor",
    "type": "shell",
    "command": "cargo-runner",
    "args": ["show", "${file}:${lineNumber}"],
    "env": {
        "CARGO_RUNNER_QUICK": "1"
    },
    "presentation": {
        "reveal": "always",
        "panel": "new"
    }
}
```

### Shell Function

Add to your `.bashrc` or `.zshrc`:

```bash
# Run test at line
crt() {
    if [ -z "$1" ]; then
        echo "Usage: crt <file:line>"
        return 1
    fi
    
    cmd=$(CARGO_RUNNER_QUICK=1 cargo-runner show "$1")
    if [ $? -eq 0 ]; then
        echo "Running: $cmd"
        eval "$cmd"
    else
        echo "No runnable found at $1"
    fi
}

# Example: crt src/lib.rs:42
```

## Advanced Usage

### Finding tests in a specific module

```bash
# Show all tests in a test module
cargo-runner show src/tests/mod.rs

# Find a specific test
cargo-runner show src/tests/integration.rs:200
```

### Working with benchmarks

```bash
# Find benchmark at line
cargo-runner show benches/performance.rs:45

# List all benchmarks
cargo-runner show benches/performance.rs | grep "Type: Benchmark"
```

### Nested test modules

The tool correctly handles nested test modules:

```bash
$ cargo-runner show src/nested/tests.rs:150
🎯 Best runnable at line 150:

📦 Run test 'test_nested_function'
📏 Scope: lines 148-152
🚀 Command: cargo test --package my_crate -- my_crate::nested::tests::inner::test_nested_function --exact
```

## Exit Codes

- `0`: Success - runnable found and displayed
- `1`: Error - no runnable found at specified line (in quick mode) or invalid arguments

## Performance

The tool uses caching to improve performance on repeated queries. The cache is automatically invalidated when files change.

## Limitations

- Only works with Rust files
- Requires valid Rust syntax (tree-sitter parsing)
- Line numbers are 1-based (matching editor display)