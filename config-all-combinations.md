# All Possible Config Combinations & Use Cases

## 1. MINIMAL CONFIGS

### 1.1 Just Change Test Runner
```json
{
  "test": {
    "cargo": { "subcommand": "nextest run" }
  }
}
```
**Use case**: Developer wants to use nextest for all tests

### 1.2 Just Add Environment Variable
```json
{
  "defaults": {
    "env": { "RUST_LOG": "debug" }
  }
}
```
**Use case**: Enable debug logging for all commands

### 1.3 Just Use Nightly
```json
{
  "defaults": {
    "channel": "nightly"
  }
}
```
**Use case**: Project requires nightly features

## 2. SINGLE BUILD SYSTEM CONFIGS

### 2.1 Cargo-Only Project
```json
{
  "test": {
    "cargo": {
      "subcommand": "nextest run",
      "args": ["--workspace"],
      "env": { "NEXTEST_PROFILE": "ci" }
    }
  },
  "binary": {
    "cargo": {
      "args": ["--release"],
      "features": "all"
    }
  },
  "bench": {
    "cargo": {
      "args": ["--features", "bench-utils"]
    }
  }
}
```
**Use case**: Typical Rust workspace with custom test/bench setup

### 2.2 Bazel Monorepo
```json
{
  "defaults": {
    "bazel": {
      "workspace": "mycompany"
    }
  },
  "test": {
    "bazel": {
      "args": ["--test_output=streamed", "--test_timeout=300"],
      "test_args": ["--exact", "{test_filter}"],
      "target_template": "//src/{package}:{file_stem}_test"
    }
  },
  "binary": {
    "bazel": {
      "target_template": "//src/{package}:{file_stem}",
      "exec_args": ["--config", "production"]
    }
  }
}
```
**Use case**: Large company monorepo with standardized Bazel setup

### 2.3 Rustc Standalone Scripts
```json
{
  "test": {
    "rustc": {
      "build": {
        "args": ["--test", "-C", "opt-level=2"],
        "env": { "RUSTFLAGS": "-C target-cpu=native" }
      },
      "exec": {
        "args": ["{test_filter}", "--nocapture"]
      }
    }
  },
  "binary": {
    "rustc": {
      "build": {
        "args": ["-C", "opt-level=3", "-C", "lto=fat"]
      },
      "exec": {
        "pipe": "| tee output.log"
      }
    }
  }
}
```
**Use case**: Competitive programming or single-file tools

## 3. MIXED BUILD SYSTEM CONFIGS

### 3.1 Migration from Cargo to Bazel
```json
{
  "test": {
    "cargo": {
      "subcommand": "test",
      "args": ["--lib"]
    },
    "bazel": {
      "args": ["--test_output=all"],
      "target_template": "//src:lib_test"
    }
  },
  "overrides": [
    {
      "match": { "path": "legacy/*" },
      "build_system": "cargo"  // Force cargo for legacy code
    },
    {
      "match": { "path": "new/*" },
      "build_system": "bazel"  // Force bazel for new code
    }
  ]
}
```
**Use case**: Gradual migration between build systems

### 3.2 Polyglot Project
```json
{
  "defaults": {
    "env": { "LOG_LEVEL": "info" }
  },
  "test": {
    "cargo": { "subcommand": "test" },
    "bazel": { 
      "args": ["--test_output=errors"],
      "target_template": "//{lang}/{package}:test"
    }
  },
  "overrides": [
    {
      "match": { "path": "rust/*", "type": "test" },
      "build_system": "cargo"
    },
    {
      "match": { "path": "cpp/*", "type": "test" },
      "build_system": "bazel"
    }
  ]
}
```
**Use case**: Multi-language project with different build systems

## 4. FEATURE-SPECIFIC CONFIGS

### 4.1 Feature Combinations
```json
{
  "test": {
    "cargo": {
      "features": "all"  // or: ["feat1", "feat2"]
    }
  },
  "overrides": [
    {
      "match": { "path": "crates/web/*" },
      "test": {
        "cargo": { "features": ["web", "ssr"] }
      }
    },
    {
      "match": { "path": "crates/cli/*" },
      "test": {
        "cargo": { "features": ["cli", "terminal"] }
      }
    }
  ]
}
```
**Use case**: Workspace with crate-specific features

### 4.2 Platform-Specific Configs
```json
{
  "overrides": [
    {
      "match": { "path": "src/windows/*" },
      "test": {
        "cargo": {
          "target": "x86_64-pc-windows-msvc",
          "env": { "WINDOWS_TEST": "1" }
        }
      }
    },
    {
      "match": { "path": "src/wasm/*" },
      "test": {
        "cargo": {
          "target": "wasm32-unknown-unknown",
          "runner": "wasm-bindgen-test-runner"
        }
      }
    }
  ]
}
```
**Use case**: Cross-platform library with platform-specific code

## 5. GRANULAR OVERRIDES

### 5.1 Function-Level Overrides
```json
{
  "test": {
    "cargo": { "subcommand": "test" }
  },
  "overrides": [
    {
      "match": {
        "function": "test_database_*",
        "type": "test"
      },
      "env": { 
        "DATABASE_URL": "postgres://test@localhost/test",
        "TEST_ISOLATION": "true"
      },
      "cargo": {
        "args": ["--test-threads=1"]
      }
    },
    {
      "match": {
        "function": "test_flaky_*",
        "type": "test"
      },
      "cargo": {
        "retry": 3,
        "timeout": "5m"
      }
    }
  ]
}
```
**Use case**: Database tests need serial execution, flaky tests need retries

### 5.2 Module-Level Overrides
```json
{
  "overrides": [
    {
      "match": {
        "module": "*::integration::*",
        "type": "test"
      },
      "env": {
        "TEST_ENV": "integration",
        "EXTERNAL_SERVICE_URL": "http://localhost:8080"
      }
    },
    {
      "match": {
        "module": "*::unit::*",
        "type": "test"
      },
      "cargo": {
        "args": ["--lib"]  // Only run lib tests
      }
    }
  ]
}
```
**Use case**: Different test types need different configs

### 5.3 File-Pattern Overrides
```json
{
  "overrides": [
    {
      "match": { "path": "**/*_test.rs" },
      "test": {
        "cargo": { "subcommand": "nextest run" }
      }
    },
    {
      "match": { "path": "**/examples/*.rs" },
      "binary": {
        "cargo": { 
          "subcommand": "run --example",
          "args": ["--features", "examples"]
        }
      }
    },
    {
      "match": { "path": "**/benches/*.rs" },
      "bench": {
        "cargo": {
          "args": ["--features", "bench"],
          "env": { "BENCH_PROFILE": "accurate" }
        }
      }
    }
  ]
}
```
**Use case**: Different file patterns have different requirements

## 6. COMPLEX REAL-WORLD CONFIGS

### 6.1 Enterprise Monorepo
```json
{
  "defaults": {
    "channel": "stable",
    "env": {
      "COMPANY_ENV": "development",
      "RUST_LOG": "warn"
    }
  },
  "test": {
    "cargo": {
      "subcommand": "nextest run",
      "args": ["--workspace"],
      "env": { "NEXTEST_PROFILE": "ci" }
    },
    "bazel": {
      "args": ["--test_output=errors", "--test_tag_filters=-flaky"],
      "target_template": "//{team}/{service}:all_tests"
    }
  },
  "binary": {
    "cargo": {
      "args": ["--release"],
      "features": ["production"]
    },
    "bazel": {
      "args": ["--compilation_mode=opt"],
      "target_template": "//{team}/{service}:server"
    }
  },
  "overrides": [
    {
      "match": { "path": "services/auth/*" },
      "test": {
        "env": {
          "AUTH_TEST_MODE": "true",
          "JWT_SECRET": "test-secret"
        },
        "cargo": {
          "features": ["auth-testing"]
        }
      }
    },
    {
      "match": { "path": "libs/database/*", "type": "test" },
      "env": {
        "DATABASE_URL": "postgres://test@db-test:5432/test",
        "RUN_MIGRATION": "true"
      },
      "cargo": {
        "args": ["--test-threads=1"]
      }
    },
    {
      "match": { 
        "function": "bench_*",
        "type": "bench"
      },
      "cargo": {
        "args": ["--features", "bench-utils"],
        "env": { 
          "BENCH_ITERATIONS": "1000",
          "BENCH_WARMUP": "100"
        }
      }
    }
  ]
}
```
**Use case**: Large enterprise with mixed build systems and complex requirements

### 6.2 Open Source Library
```json
{
  "defaults": {
    "channel": "stable"
  },
  "test": {
    "cargo": {
      "subcommand": "test",
      "args": ["--all-features"]
    }
  },
  "doc": {
    "cargo": {
      "subcommand": "test",
      "args": ["--doc", "--all-features"]
    }
  },
  "overrides": [
    {
      "match": { "path": "tests/compatibility/*" },
      "test": {
        "matrix": [
          { "channel": "stable" },
          { "channel": "beta" },
          { "channel": "nightly" },
          { "channel": "1.70.0" }  // MSRV
        ]
      }
    },
    {
      "match": { "feature": "no_std" },
      "test": {
        "cargo": {
          "args": ["--no-default-features", "--features", "no_std"]
        }
      }
    }
  ]
}
```
**Use case**: Library that needs to test across multiple Rust versions and feature sets

### 6.3 ML/Scientific Computing Project
```json
{
  "defaults": {
    "env": {
      "RUST_LOG": "info",
      "OMP_NUM_THREADS": "8"
    }
  },
  "test": {
    "cargo": {
      "args": ["--release"],  // Tests need optimization
      "features": ["test-utils"]
    }
  },
  "bench": {
    "cargo": {
      "args": ["--features", "bench,cuda,mkl"],
      "env": {
        "BENCH_DATASET": "medium",
        "CUDA_VISIBLE_DEVICES": "0"
      }
    }
  },
  "overrides": [
    {
      "match": { "path": "src/gpu/*" },
      "test": {
        "cargo": {
          "features": ["cuda"],
          "env": { "CUDA_TEST": "1" }
        },
        "require": ["nvidia-smi"]  // Pre-flight check
      }
    },
    {
      "match": { "function": "test_accuracy_*" },
      "test": {
        "cargo": {
          "args": ["--release", "--", "--test-threads=1"],
          "timeout": "30m",
          "env": { "TOLERANCE": "1e-6" }
        }
      }
    }
  ]
}
```
**Use case**: ML project with GPU code and long-running accuracy tests

## 7. SPECIAL CASES

### 7.1 Single File Scripts
```json
{
  "binary": {
    "single_file": {
      "rustc": {
        "build": {
          "args": ["-C", "opt-level=3"]
        }
      }
    }
  },
  "overrides": [
    {
      "match": { "path": "scripts/*" },
      "binary": {
        "single_file": {
          "shebang": true,  // Add #!/usr/bin/env cargo-runner
          "rustc": {
            "build": {
              "args": ["--edition", "2021"]
            }
          }
        }
      }
    }
  ]
}
```
**Use case**: Collection of standalone Rust scripts

### 7.2 Doctest Configuration
```json
{
  "doc": {
    "cargo": {
      "args": ["--doc", "--no-fail-fast"],
      "env": { "RUST_TEST_THREADS": "1" }
    }
  },
  "overrides": [
    {
      "match": { "path": "src/examples.rs" },
      "doc": {
        "cargo": {
          "features": ["examples"],
          "env": { "DOC_TEST_SHOW_OUTPUT": "1" }
        }
      }
    }
  ]
}
```
**Use case**: Documentation with executable examples

### 7.3 Cross-Compilation
```json
{
  "defaults": {
    "cargo": {
      "target": "x86_64-unknown-linux-gnu"
    }
  },
  "overrides": [
    {
      "match": { "path": "embedded/*" },
      "binary": {
        "cargo": {
          "target": "thumbv7m-none-eabi",
          "runner": "probe-run",
          "env": { "DEFMT_LOG": "trace" }
        }
      }
    },
    {
      "match": { "path": "wasm/*" },
      "test": {
        "cargo": {
          "target": "wasm32-unknown-unknown",
          "runner": "wasm-bindgen-test-runner"
        }
      }
    }
  ]
}
```
**Use case**: Project targeting multiple platforms

## 8. INHERITANCE EXAMPLES

### 8.1 Multi-Level Inheritance
```json
{
  // Level 1: Global defaults
  "defaults": {
    "env": { "COMPANY": "acme" },
    "channel": "stable"
  },
  
  // Level 2: Runnable type defaults
  "test": {
    "cargo": {
      "subcommand": "nextest run",
      "env": { "TEST_MODE": "true" }  // Adds to defaults
    }
  },
  
  // Level 3: File overrides
  "overrides": [{
    "match": { "path": "integration/*" },
    "test": {
      "cargo": {
        "env": { 
          "TEST_MODE": "integration",  // Overrides level 2
          "DB_URL": "test://db"        // Adds new
        }
        // channel: "stable" still inherited from level 1
      }
    }
  }]
}
```
**Result for integration test**:
- env: `{ COMPANY: "acme", TEST_MODE: "integration", DB_URL: "test://db" }`
- channel: `"stable"`
- subcommand: `"nextest run"`

### 8.2 Build System Selection
```json
{
  "test": {
    "cargo": { "subcommand": "test" },
    "bazel": { "args": ["--test_output=all"] }
  },
  "overrides": [
    {
      "match": { "path": "rs/*" },
      "prefer": "cargo"  // Hint which build system to use
    },
    {
      "match": { "path": "cc/*" },
      "prefer": "bazel"
    }
  ]
}
```

## Key Observations

1. **Runnable-type organization** makes configs more intuitive
2. **Inheritance** dramatically reduces repetition
3. **Override matching** is flexible (path, function, module, type)
4. **Build system agnostic** at the top level
5. **Real-world configs** often need complex overrides
6. **Special cases** (like single-file scripts) fit naturally

This structure handles everything from simple "use nextest" to complex enterprise monorepos with multiple build systems and intricate requirements.