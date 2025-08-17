# Universal Command Runner 🚀

A language-agnostic command runner that can detect and execute runnable code in any programming language.

## ✅ Installation & Testing

### Quick Install

```bash
# From the command_runner directory
cd /Users/uriah/Code/windrunner/command_runner

# Install locally (adds to ~/.cargo/bin)
cargo install --path .

# Verify installation
runner --version
# Output: runner 0.1.0
```

### Build from Source

```bash
# Build debug version
cargo build

# Build optimized release version  
cargo build --release

# Run without installing
cargo run -- analyze test_files/example.rs
```

## 🎯 Usage Examples

### List Available Plugins

```bash
runner plugin-list
# Output:
# Available plugins:
#   - rust
#   - python
#   - javascript
```

### Analyze Files

```bash
# Analyze a Rust file
runner analyze test_files/example.rs
# Output: Found 5 runnables:
#   1. Run main (lines 3-4)
#   2. Test: test_addition (lines 13-18)
#   ...

# Analyze a Python file
runner analyze test_files/example.py
# Output: Found 9 runnables:
#   1. Test: test_addition (lines 14-24)
#   2. Test: test_subtraction (lines 18-28)
#   ...

# Analyze a JavaScript file
runner analyze test_files/example.js
# Output: Found 7 runnables:
#   1. Test (lines 14-19)
#   2. Test (lines 18-23)
#   ...
```

### Run Code at Specific Line

```bash
# Run test at line 15 in a Rust file
runner run test_files/example.rs:15
# Executes: cargo test test_addition -- --exact

# Run Python script
runner run test_files/example.py:55
# Executes: python test_files/example.py

# Run JavaScript test at line 20
runner test test_files/example.js:20
# Executes: npm test -- test_files/example.js
```

## 🧪 Testing

### Run Tests

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific functionality
cargo test test_rust_plugin
```

### Test Script

```bash
# Make test script executable
chmod +x test.sh

# Run comprehensive tests
./test.sh
```

## 📁 Project Structure

```
command_runner/
├── src/
│   ├── lib.rs       # Core library with plugins
│   └── main.rs      # CLI binary
├── test_files/      # Example files for testing
│   ├── example.rs   # Rust test file
│   ├── example.py   # Python test file
│   └── example.js   # JavaScript test file
├── Cargo.toml       # Project configuration
├── README.md        # This file
├── INSTALL.md       # Detailed installation guide
├── ARCHITECTURE.md  # System architecture
├── PLUGIN_GUIDE.md  # Plugin development guide
└── BUILD_GUIDE.md   # Build instructions
```

## 🔌 Supported Languages

Currently built-in support for:
- **Rust** - Detects tests, benchmarks, main functions
- **Python** - Detects pytest/unittest tests, main blocks
- **JavaScript/TypeScript** - Detects Jest/Mocha tests

## 🛠️ How It Works

1. **File Analysis**: Reads source files and detects runnable items
2. **Plugin Selection**: Automatically selects the right language plugin
3. **Command Building**: Constructs appropriate commands (cargo, python, npm, etc.)
4. **Execution**: Runs the command with proper working directory

## 📝 Commands

### Main Commands

- `runner run <file[:line]>` - Run code at specific location
- `runner test <file[:line]>` - Run tests
- `runner analyze <file>` - List all runnables in a file
- `runner plugin-list` - List available plugins

### Examples

```bash
# Run main function
runner run test_files/example.rs

# Run specific test
runner test test_files/example.py:15

# Analyze what can be run
runner analyze test_files/example.js
```

## 🚀 Future Features

- [ ] WASM plugin loading for security
- [ ] Plugin registry and distribution
- [ ] More language plugins (Go, Java, Ruby, etc.)
- [ ] Configuration files
- [ ] LSP integration
- [ ] Debug support

## 🤝 Contributing

This is a proof-of-concept for a universal command runner. The architecture supports:

1. **Adding new languages** - Implement the `LanguagePlugin` trait
2. **Improving detection** - Enhance runnable detection logic
3. **Better command building** - Add framework-specific commands

## 📄 License

MIT OR Apache-2.0

## 🆚 Comparison with Windrunner

| Feature | Windrunner | Command Runner |
|---------|------------|----------------|
| Languages | Rust only | Any language |
| Architecture | Monolithic | Plugin-based |
| Extensibility | Limited | Unlimited |
| Scope | Rust testing | Universal runner |

## ✨ Key Innovation

Unlike traditional language-specific runners, this framework provides:
- **One interface** for all languages
- **Consistent behavior** across different ecosystems
- **Extensible plugin system** for community contributions
- **Future-proof architecture** supporting new languages

---

Built as a standalone project to demonstrate universal command running across any programming language.