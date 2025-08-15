# Cargo Runner Configuration Analysis

## Complete `.cargo-runner.json` Example

This document provides a comprehensive example of all possible configuration options for cargo-runner.

```json
{
  "cargo": {
    "command": "cargo",
    "subcommand": "test",
    "channel": "nightly",
    "features": "all",
    "extra_args": ["--release", "--verbose"],
    "extra_env": {
      "RUST_LOG": "debug",
      "RUST_BACKTRACE": "1"
    },
    "extra_test_binary_args": ["--nocapture", "--test-threads=1"],
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest",
      "channel": "nightly",
      "features": ["test-feature", "integration"],
      "extra_args": ["run"],
      "extra_env": {
        "NEXTEST_PROFILE": "ci"
      }
    },
    "binary_framework": {
      "command": "cargo",
      "subcommand": "run",
      "channel": "stable",
      "features": ["production"],
      "extra_args": ["--release"],
      "extra_env": {
        "ENV": "production"
      }
    },
    "linked_projects": ["../dependency", "../shared"],
    "package": "my-package"
  },
  
  "bazel": {
    "workspace": "my_workspace",
    "default_test_target": "//:test",
    "default_binary_target": "//:server",
    "test_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed", "--test_timeout", "300"],
      "extra_args": ["--test_env=RUST_LOG=debug"],
      "test_args": ["--exact", "{test_filter}"],
      "exec_args": [],
      "extra_env": {
        "BAZEL_TEST": "1"
      }
    },
    "binary_framework": {
      "command": "bazel",
      "subcommand": "run",
      "target": "{target}",
      "args": ["--compilation_mode", "opt"],
      "extra_args": [],
      "test_args": [],
      "exec_args": ["--port", "8080"],
      "extra_env": {
        "BAZEL_RUN": "1"
      }
    },
    "benchmark_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed", "--test_arg", "--bench"],
      "extra_args": ["--test_arg", "--nocapture"],
      "test_args": ["{bench_filter}"],
      "exec_args": [],
      "extra_env": {
        "BENCH_MODE": "1"
      }
    },
    "doc_test_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed"],
      "extra_args": [],
      "test_args": [],
      "exec_args": [],
      "extra_env": {}
    },
    "test_target": "//src:test",
    "binary_target": "//src:main",
    "extra_test_args": ["--test_output=all"],
    "extra_run_args": ["--verbose"],
    "extra_test_binary_args": ["--test-threads=4"],
    "extra_env": {
      "BAZEL_GLOBAL": "1"
    }
  },
  
  "rustc": {
    "test_framework": {
      "build": {
        "command": "rustc",
        "args": ["--test", "-o", "{output}"],
        "extra_args": ["-C", "opt-level=3"],
        "extra_test_binary_args": [],
        "pipe": null,
        "suppress_stderr": false,
        "extra_env": {
          "RUSTFLAGS": "-C target-cpu=native"
        }
      },
      "exec": {
        "command": "{output}",
        "args": [],
        "extra_args": ["--nocapture"],
        "extra_test_binary_args": ["--test-threads=1"],
        "pipe": null,
        "suppress_stderr": false,
        "extra_env": {
          "RUST_TEST_THREADS": "1"
        }
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "args": ["-o", "{output}"],
        "extra_args": ["-C", "opt-level=3", "-C", "lto=fat"],
        "extra_test_binary_args": [],
        "pipe": null,
        "suppress_stderr": false,
        "extra_env": {
          "RUSTFLAGS": "-C target-cpu=native"
        }
      },
      "exec": {
        "command": "{output}",
        "args": [],
        "extra_args": ["--port", "8080"],
        "extra_test_binary_args": [],
        "pipe": "| tee output.log",
        "suppress_stderr": false,
        "extra_env": {
          "APP_ENV": "production"
        }
      }
    },
    "benchmark_framework": {
      "build": {
        "command": "rustc",
        "args": ["--test", "-o", "{output}", "-C", "opt-level=3"],
        "extra_args": [],
        "extra_test_binary_args": [],
        "pipe": null,
        "suppress_stderr": false,
        "extra_env": {}
      },
      "exec": {
        "command": "{output}",
        "args": ["--bench"],
        "extra_args": [],
        "extra_test_binary_args": [],
        "pipe": null,
        "suppress_stderr": false,
        "extra_env": {}
      }
    }
  },
  
  "single_file_script": {
    "extra_args": ["--verbose"],
    "extra_env": {
      "RUST_LOG": "debug"
    },
    "extra_test_binary_args": ["--nocapture"]
  },
  
  "overrides": [
    {
      "match": {
        "package": "my_crate",
        "module_path": "my_crate::tests::*",
        "file_path": "src/tests/integration.rs",
        "function_name": "test_*",
        "file_type": "cargo_project"
      },
      "cargo": {
        "command": "cargo",
        "subcommand": "nextest",
        "channel": "nightly",
        "features": ["integration", "test"],
        "extra_args": ["run", "--profile", "ci"],
        "extra_env": {
          "TEST_ENV": "integration"
        },
        "extra_test_binary_args": ["--test-threads=1"],
        "test_framework": {
          "command": "cargo",
          "subcommand": "nextest",
          "channel": "nightly",
          "features": "all",
          "extra_args": ["run"],
          "extra_env": {
            "NEXTEST_PROFILE": "integration"
          }
        }
      },
      "bazel": {
        "test_framework": {
          "command": "bazel",
          "subcommand": "test",
          "target": "//tests:integration_test",
          "args": ["--test_output", "all"],
          "test_args": ["--exact", "{test_filter}"]
        }
      },
      "rustc": {
        "test_framework": {
          "build": {
            "command": "rustc",
            "args": ["--test", "-o", "{output}", "--cfg", "integration"],
            "extra_env": {
              "RUSTFLAGS": "-C opt-level=2"
            }
          },
          "exec": {
            "command": "{output}",
            "args": ["--test-threads=1"],
            "extra_env": {
              "TEST_MODE": "integration"
            }
          }
        }
      },
      "single_file_script": {
        "extra_args": ["--integration"],
        "extra_env": {
          "SCRIPT_MODE": "test"
        }
      }
    },
    {
      "match": {
        "module_path": "*::benchmarks::*",
        "function_name": "bench_*",
        "file_type": "cargo_project"
      },
      "cargo": {
        "subcommand": "bench",
        "extra_args": ["--bench", "main"],
        "extra_test_binary_args": ["--nocapture"]
      }
    },
    {
      "match": {
        "file_path": "examples/*.rs",
        "file_type": "single_file_script"
      },
      "single_file_script": {
        "extra_args": ["--example"],
        "extra_env": {
          "EXAMPLE": "true"
        }
      }
    }
  ]
}
```

## Configuration Structure Breakdown

### 1. **Top-level Build System Configurations**

#### `cargo` - Cargo Build System
- **command**: The cargo executable (default: "cargo")
- **subcommand**: Default subcommand (e.g., "test", "run", "bench")
- **channel**: Rust channel ("nightly", "stable", "beta")
- **features**: Either `"all"` for --all-features or array of feature names
- **extra_args**: Additional arguments passed to cargo
- **extra_env**: Environment variables
- **extra_test_binary_args**: Arguments passed after `--` to test binary
- **test_framework**: Override configuration for test runs
- **binary_framework**: Override configuration for binary runs
- **linked_projects**: Array of paths to linked projects
- **package**: Package name for workspace projects

#### `bazel` - Bazel Build System
- **workspace**: Bazel workspace name
- **default_test_target**: Default target for tests
- **default_binary_target**: Default target for binaries
- **test_framework**: Configuration for test runs
- **binary_framework**: Configuration for binary runs
- **benchmark_framework**: Configuration for benchmark runs
- **doc_test_framework**: Configuration for doc test runs
- **test_target**: Legacy field for backward compatibility
- **binary_target**: Legacy field for backward compatibility
- **extra_test_args**: Legacy field for backward compatibility
- **extra_run_args**: Legacy field for backward compatibility
- **extra_test_binary_args**: Legacy field for backward compatibility
- **extra_env**: Global environment variables

#### `rustc` - Direct Rustc Compilation
- **test_framework**: Two-phase config (build + exec) for tests
- **binary_framework**: Two-phase config (build + exec) for binaries
- **benchmark_framework**: Two-phase config (build + exec) for benchmarks

Each framework has:
- **build**: Compilation phase configuration
  - **command**: Compiler command
  - **args**: Base arguments with placeholders
  - **extra_args**: Additional arguments
  - **pipe**: Shell pipe command
  - **suppress_stderr**: Hide stderr output
  - **extra_env**: Build environment variables
- **exec**: Execution phase configuration
  - **command**: Executable command (often `{output}`)
  - **args**: Base execution arguments
  - **extra_args**: Additional execution arguments
  - **extra_test_binary_args**: Test-specific arguments
  - **pipe**: Output pipe command
  - **suppress_stderr**: Hide stderr output
  - **extra_env**: Runtime environment variables

#### `single_file_script` - Standalone Scripts
- **extra_args**: Additional arguments for script execution
- **extra_env**: Environment variables
- **extra_test_binary_args**: Test-specific arguments

### 2. **Overrides Section**

The `overrides` array allows specific configurations based on matching criteria:

#### Match Criteria (`match` object)
- **package**: Package name to match
- **module_path**: Module path (supports wildcards with `*`)
- **file_path**: File path (supports glob patterns)
- **function_name**: Function name (supports wildcards)
- **file_type**: One of:
  - `"cargo_project"`: Files in a Cargo project
  - `"standalone"`: Standalone Rust files
  - `"single_file_script"`: Single-file Rust scripts

#### Override Configurations
Each override can specify configurations for:
- **cargo**: Cargo-specific overrides
- **bazel**: Bazel-specific overrides
- **rustc**: Rustc-specific overrides
- **single_file_script**: Script-specific overrides

### 3. **Framework Configurations**

#### Test Framework (`test_framework`)
Used when running tests. Available in:
- `cargo.test_framework`
- `bazel.test_framework`
- `rustc.test_framework`

#### Binary Framework (`binary_framework`)
Used when running binaries. Available in:
- `cargo.binary_framework`
- `bazel.binary_framework`
- `rustc.binary_framework`

#### Benchmark Framework (`benchmark_framework`)
Used for benchmarks. Available in:
- `bazel.benchmark_framework`
- `rustc.benchmark_framework`

#### Doc Test Framework (`doc_test_framework`)
Used for doc tests. Available in:
- `bazel.doc_test_framework`

### 4. **Placeholder Variables**

Different build systems support different placeholders:

#### Bazel Placeholders
- `{target}`: The detected or configured Bazel target
- `{test_filter}`: Test function name for filtering
- `{bench_filter}`: Benchmark function name for filtering
- `{file_name}`: Current file name without extension

#### Rustc Placeholders
- `{output}`: Output binary path

### 5. **Features Configuration**

The `features` field can be:
- `"all"`: Translates to `--all-features`
- `["feature1", "feature2"]`: Translates to `--features=feature1,feature2`

### 6. **Configuration Precedence**

1. Function-specific overrides (most specific)
2. Build system framework configurations
3. Build system default configurations
4. Global defaults (least specific)

### 7. **Use Cases**

#### Running Tests with Nextest
```json
{
  "cargo": {
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest",
      "extra_args": ["run"]
    }
  }
}
```

#### Bazel Integration Tests
```json
{
  "overrides": [{
    "match": {
      "file_path": "tests/integration/*.rs"
    },
    "bazel": {
      "test_framework": {
        "target": "//tests/integration:all",
        "args": ["--test_output", "all"]
      }
    }
  }]
}
```

#### Custom Rustc Build
```json
{
  "rustc": {
    "binary_framework": {
      "build": {
        "args": ["-C", "target-cpu=native", "-C", "lto=fat", "-o", "{output}"]
      }
    }
  }
}
```

#### Environment-Specific Configuration
```json
{
  "overrides": [{
    "match": {
      "module_path": "*::integration::*"
    },
    "cargo": {
      "extra_env": {
        "DATABASE_URL": "postgres://test@localhost/test"
      }
    }
  }]
}
```

## Configuration File Locations

Cargo-runner looks for configuration files in this order:
1. `.cargo-runner.json` in the current directory
2. `cargo-runner.json` in the current directory
3. Walks up the directory tree looking for these files
4. Uses `PROJECT_ROOT` environment variable if set

## Notes

- The `cargo runner override` command mentioned in the CLI is deprecated
- Use the configuration file for all overrides
- Configuration merging happens from global to specific
- All fields are optional - only specify what you need to override
- Legacy Bazel fields are maintained for backward compatibility but new configurations should use the framework approach