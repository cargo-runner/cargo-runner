# Scope-Based Runnable Detection and Command Building Guide

## Overview

The cargo-runner uses a sophisticated scope detection system combined with pattern matching to identify runnable code and build appropriate cargo commands. This guide explains how scope ranges work and how they interact with our runnable detection system.

## Understanding Scope Ranges

### What is a Scope?

A scope represents a contiguous range of lines in a Rust source file that forms a logical unit. Each scope has:
- **Start and end positions** (line and character)
- **A type** (file, module, struct, impl, function, etc.)
- **Parent-child relationships** with other scopes

```rust
#[derive(Debug, Clone)]
pub struct Scope {
    pub start: Position,
    pub end: Position,
}

pub struct Position {
    pub line: u32,
    pub character: u32,
}
```

### Scope Hierarchy Example

```rust
// <!- File Scope Start (Line 1) -!>
use std::string::String;

/// User struct documentation
/// This documentation extends the struct's scope range
struct User {
    pub name: String,
    pub age: u8,
}
// <!- User Struct Scope End (Line 9) -!>

/// Implementation block for User
/// This also extends the impl scope range
impl User {
    /// Creates a new User instance
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use project_a::User;
    /// let user = User::new(String::from("Uriah"), 36);
    /// assert_eq!(user.name, String::from("Uriah"));
    /// assert_eq!(user.age, 36);
    /// ```
    pub fn new(name: String, age: u8) -> Self {
        Self { name, age }
    } // <!- new() Function Scope End (Line 27) -!>
    
    /// Gets the user's name
    pub fn name(&self) -> &str {
        &self.name
    } // <!- name() Function Scope End (Line 33) -!>
} // <!- impl User Scope End (Line 34) -!>

fn main() {
    let user = User::new(String::from("Test"), 25);
    println!("{}", user.name());
} // <!- main() Function Scope End (Line 40) -!>

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User::new(String::from("Test"), 30);
        assert_eq!(user.name(), "Test");
    } // <!- test_user_creation() Scope End (Line 50) -!>
    
    #[test]
    fn test_user_age() {
        let user = User::new(String::from("Test"), 30);
        assert_eq!(user.age, 30);
    } // <!- test_user_age() Scope End (Line 57) -!>
} // <!- mod tests Scope End (Line 58) -!>
// <!- File Scope End (Line 59) -!>
```

### Scope Ranges in This Example:

1. **File Scope**: Lines 1-59 (entire file)
2. **Struct Scope**: Lines 3-9 (`struct User`)
3. **Impl Scope**: Lines 11-34 (`impl User`)
4. **Function Scopes**:
   - Lines 14-27: `User::new()`
   - Lines 29-33: `User::name()`
   - Lines 37-40: `main()`
   - Lines 46-50: `test_user_creation()`
   - Lines 52-57: `test_user_age()`
5. **Module Scope**: Lines 42-58 (`mod tests`)

## How Documentation Comments Extend Scopes

Our scope detector recognizes that `///` documentation comments extend the scope of the item they document:

```rust
/// This comment extends the function's scope upward
/// So the function scope actually starts here, not at 'fn'
/// 
/// Even blank doc comment lines extend the scope
fn documented_function() {
    // function body
}
```

This is crucial for doc test detection, as doc tests are part of the function's documentation scope.

## Runnable Detection Patterns

Our system detects several patterns that indicate runnable code:

### 1. Doc Test Pattern
```rust
/// ```rust
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
```
- **Detected by**: `DocTestPattern`
- **Command**: `cargo test --doc --package {package} -- {struct_or_module_name}`

### 2. Test Function Pattern
```rust
#[test]
fn test_something() { }

#[tokio::test]
async fn test_async() { }

#[cfg(test)]
mod tests {
    #[test]
    fn inner_test() { }
}
```
- **Detected by**: `TestPattern`
- **Commands**:
  - Specific test: `cargo test --package {package} -- {module_path}::{test_name} --exact`
  - Module tests: `cargo test --package {package} -- {module_path}`

### 3. Benchmark Pattern
```rust
#[bench]
fn bench_something(b: &mut Bencher) { }

// Criterion benchmarks
fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(20)));
}
```
- **Detected by**: `BenchPattern`
- **Command**: `cargo bench --package {package} --bench {bench_name}`

### 4. Binary/Main Function Pattern
```rust
fn main() {
    println!("Hello, world!");
}
```
- **Detected by**: `BinaryPattern`
- **Commands**:
  - Main binary: `cargo run --package {package}`
  - Named binary: `cargo run --package {package} --bin {binary_name}`

## Scope Resolution and Priority Rules

When multiple scopes contain a given line, we use these priority rules:

### 1. Most Specific Scope Wins
```rust
impl User {                    // Lines 10-33
    fn new() -> Self {         // Lines 14-30
        // Line 20 is here
    }
}
```
- Line 20 belongs to both `impl User` and `fn new()` scopes
- We select `fn new()` because it's more specific (smaller range)

### 2. Runnable Type Priority
When multiple runnables contain the same line:
1. **Doc tests with method specifiers** (highest priority)
2. **Specific test functions**
3. **Specific benchmarks**
4. **Module-level test runners** (lowest priority)

```rust
#[cfg(test)]
mod tests {                    // Module-level runner
    #[test]
    fn specific_test() {       // Specific test runner
        // Line here
    }
}
```

### 3. Range Size Resolution
When runnable types are equal, smaller ranges win:
```rust
// RunnableWithScore sorting
scored_runnables.sort_by(|a, b| {
    // First check runnable type priority
    if a.is_module_test && !b.is_module_test {
        return Ordering::Greater; // b wins
    }
    // Then sort by range size
    a.range_size.cmp(&b.range_size)
});
```

## Command Building Process

### 1. Module Path Resolution
The module path is built from:
- Package name (from Cargo.toml)
- Module hierarchy (from file path and inline modules)
- Item name (struct, function, etc.)

Example: `my_crate::models::user::User::new`

### 2. Command Construction
```rust
// Base command determined by runnable type
let base_command = match runnable_type {
    Test => vec!["test"],
    Bench => vec!["bench"],
    Binary => vec!["run"],
    DocTest => vec!["test", "--doc"],
};

// Add package information
command.extend(["--package", package_name]);

// Add target specifier
match target {
    Lib => command.push("--lib"),
    Bin(name) => command.extend(["--bin", name]),
    Example(name) => command.extend(["--example", name]),
    Test(name) => command.extend(["--test", name]),
    Bench(name) => command.extend(["--bench", name]),
}

// Add test filter for specific tests
if let Some(test_path) = specific_test_path {
    command.push("--");
    command.push(test_path); // e.g., "tests::test_user_creation"
    command.push("--exact");
}
```

## Integration with Configuration System

The detected scopes and runnables interact with the override system:

```rust
// Function identity is built from scope information
let identity = FunctionIdentity {
    package: Some("my_package"),
    module_path: Some("my_package::user"),
    file_path: Some("/src/user.rs"),
    function_name: Some("new"),
};

// Overrides are applied based on matching scope
let command = build_command(&config, &identity, cargo_args, exec_args, &project_root);
```

## Advanced Scope Detection Features

### 1. Inline Module Detection
```rust
mod outer {
    mod inner {
        fn deeply_nested() { }
    }
}
```
The module resolver tracks inline modules to build correct paths: `outer::inner::deeply_nested`

### 2. Multiple Impl Blocks
```rust
struct User { }

impl User {
    fn method1() { }
}

impl Display for User {
    fn fmt() { }
}
```
Each impl block has its own scope, allowing precise runnable detection.

### 3. Async and Feature-Gated Tests
```rust
#[tokio::test]
async fn async_test() { }

#[cfg(feature = "integration")]
#[test]
fn integration_test() { }
```
Different test frameworks and conditional compilation are handled appropriately.

## Performance Optimizations

1. **Caching**: Parsed ASTs and detected runnables are cached
2. **Incremental Detection**: Only re-analyze changed files
3. **Parallel Processing**: Multiple files can be analyzed concurrently
4. **Early Termination**: Stop searching once the best match is found

## Debugging Scope Resolution

To debug scope resolution issues:

1. Check the detected scopes:
```rust
let context = get_scope_context(file_content, line);
println!("Current scope: {:?}", context.current_scope);
println!("All scopes at line: {:?}", context.scopes);
```

2. Verify runnable detection:
```rust
let runnables = detector.detect_runnables(file_path, Some(line))?;
for runnable in &runnables {
    println!("Found: {} at {:?}", runnable.label, runnable.scope);
}
```

3. Trace command building:
```rust
console_log!("Selected runnable: {}", runnable.label);
console_log!("Module path: {}", module_path);
console_log!("Final command: {:?}", command_args);
```

This scope-based approach ensures that cargo-runner can accurately identify what code to run and build the appropriate commands, regardless of code structure complexity.
