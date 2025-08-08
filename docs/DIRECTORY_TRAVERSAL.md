# Directory Traversal for Cargo.toml Discovery

The cargo-runner tool needs to find the appropriate `Cargo.toml` file to determine the package name for cargo commands. The directory traversal has been designed with security and performance in mind.

## Search Algorithm

Starting from the target file's directory, the tool walks up the directory tree looking for a `Cargo.toml` file. The search stops when:

1. A `Cargo.toml` file is found
2. The search boundary is reached
3. No parent directory exists

## Search Boundaries

The search boundary is determined by the following priority:

1. **PROJECT_ROOT environment variable** (if set)
   ```bash
   PROJECT_ROOT=/path/to/project cargo-x src/main.rs
   ```

2. **HOME environment variable** (Unix/Linux/macOS)
   ```bash
   echo $HOME  # Usually /home/username or /Users/username
   ```

3. **USERPROFILE environment variable** (Windows)
   ```bash
   echo %USERPROFILE%  # Usually C:\Users\username
   ```

4. **Heuristic detection** - If no environment variables are available, the tool attempts to detect the home directory by looking for `/home/username` or `/Users/username` patterns in the current path.

5. **No traversal** - If none of the above can be determined, the search returns `None` immediately.

## Security Considerations

- The tool never attempts to traverse beyond the user's home directory by default
- Setting `PROJECT_ROOT` allows restricting the search to a specific project directory
- No sudo/administrator privileges are required
- The tool respects file system permissions

## Examples

```bash
# Use default boundary (home directory)
cargo-x src/lib.rs

# Restrict to specific project
PROJECT_ROOT=/workspace/myproject cargo-x src/lib.rs

# In a workspace, finds the nearest Cargo.toml
cd /workspace/project/subcrate/src
cargo-x lib.rs  # Finds /workspace/project/subcrate/Cargo.toml
```

## Implementation Details

The `ModuleResolver::find_cargo_toml()` function implements this logic:

```rust
pub fn find_cargo_toml(start_path: &Path) -> Option<PathBuf>
```

The function uses the `cargo_toml` crate to parse the manifest and extract the package name, ensuring proper TOML parsing and error handling.