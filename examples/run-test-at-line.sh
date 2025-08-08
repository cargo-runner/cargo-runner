#!/bin/bash

# run-test-at-line.sh - Run the test at a specific line in a Rust file
#
# Usage: ./run-test-at-line.sh <file:line>
# Example: ./run-test-at-line.sh src/lib.rs:42

if [ -z "$1" ]; then
    echo "Usage: $0 <file:line>"
    echo "Example: $0 src/lib.rs:42"
    exit 1
fi

# Get the cargo command for the runnable at the specified location
cmd=$(CARGO_RUNNER_QUICK=1 cargo run --bin cargo-runner -- show "$1" 2>/dev/null)

if [ $? -eq 0 ]; then
    echo "🚀 Found runnable at $1"
    echo "📦 Running: $cmd"
    echo "─────────────────────────────────────────"
    
    # Execute the command
    eval "$cmd"
else
    echo "❌ No runnable found at $1"
    echo ""
    echo "💡 Tip: Make sure the line number points to a test, benchmark, or main function."
    echo "        You can omit the line number to see all available runnables:"
    echo "        $0 ${1%:*}"
    exit 1
fi