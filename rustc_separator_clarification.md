# Rustc Binary `--` Separator Clarification

## The Real Story

The `--` separator behavior depends on the MODE:

### 1. Regular Binary Mode (NO --test)
```bash
rustc main.rs -o myapp
./myapp -- --some-arg --another-arg
```
- The `--` is handled by YOUR program's argument parser
- Common pattern: `./myapp [OPTIONS] -- [ARGS]`
- Everything after `--` is passed to your program as-is

### 2. Test Mode (WITH --test)
```bash
rustc --test test.rs -o test_binary
./test_binary tests::test_alpha --exact --no-capture
```
- NO `--` separator needed!
- Test harness parses ALL arguments directly
- Flags like `--exact`, `--no-capture` are mixed with test names

## Examples

### Regular Binary with User Args
```rust
// main.rs
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // args might be: ["./myapp", "--", "--user-flag", "value"]
}
```

```bash
rustc main.rs -o myapp
./myapp -- --user-flag value  # -- is for YOUR program
```

### Test Binary
```bash
rustc --test test.rs -o test_bin

# All of these work WITHOUT --
./test_bin tests::test_alpha --exact
./test_bin --no-capture tests::test_alpha
./test_bin tests::test_alpha --no-capture --exact
./test_bin --list
```

## Why This Matters for Config

For our rustc configuration:

1. **Binary Mode**: 
   - `runtime_args` are passed directly to the binary
   - If the binary expects `--`, that's up to the user's program

2. **Test Mode**:
   - `runtime_args` are test harness flags
   - NO `--` separator should be added by us
   - Mix test names and flags freely

## Config Implications

```json
{
  "rustc": {
    "binary_mode": {
      "runtime_args": ["--", "--my-app-flag"]  // User adds -- if their app needs it
    },
    "test_mode": {
      "runtime_args": ["--no-capture", "--exact"]  // NO -- needed
    }
  }
}
```

## Command Generation Should Be:

### Binary Mode
```bash
# Compile
rustc file.rs -o app

# Run (runtime_args passed as-is)
./app [runtime_args from config]
```

### Test Mode  
```bash
# Compile
rustc --test file.rs -o test_bin

# Run specific test
./test_bin tests::test_name [runtime_args from config]

# Example
./test_bin tests::test_alpha --no-capture --exact
```

NO automatic `--` insertion needed!