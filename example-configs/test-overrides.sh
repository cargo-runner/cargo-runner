#!/bin/bash

# Test script to demonstrate config overrides

echo "=== Testing Config Overrides ==="
echo

# Function to test a configuration
test_config() {
    local config_file=$1
    local test_file=$2
    local line=$3
    local description=$4
    
    echo "Testing: $description"
    echo "Config: $config_file"
    echo "File: $test_file"
    
    # Copy config
    cp "$config_file" .cargo-runner.json
    
    # Run analyze with config details
    echo "Command output:"
    cargo runner analyze "$test_file" -c | grep -A 20 "Configuration Details:" | head -25
    echo
    echo "---"
    echo
}

# Navigate to nodes directory
cd ~/Code/nodes

# Test 1: Standalone Rust file
echo "=== Test 1: Standalone Rust File ==="
test_config \
    "~/Code/windrunner/example-configs/02-standalone-rust-overrides.json" \
    "test.rs:25" \
    "0" \
    "Standalone Rust file with test overrides"

# Test 2: Single File Script
echo "=== Test 2: Single File Script ==="
test_config \
    "~/Code/windrunner/example-configs/03-single-file-script-overrides.json" \
    "sfc.rs:25" \
    "0" \
    "Cargo script file with test overrides"

# Test 3: Show specific function override
echo "=== Test 3: Function-specific Override ==="
cp ~/Code/windrunner/example-configs/02-standalone-rust-overrides.json .cargo-runner.json
echo "Analyzing test_alpha function:"
cargo runner analyze test.rs:9 -c | grep -A 30 "test_alpha"

# Clean up
rm -f .cargo-runner.json

echo
echo "=== Testing Complete ==="