#!/bin/bash
# Wrapper script to run tests with JSON output piped to jq

# First argument is the test binary
TEST_BIN="$1"
shift

# Run the test binary with all remaining arguments and pipe to jq
"$TEST_BIN" "$@" | jq