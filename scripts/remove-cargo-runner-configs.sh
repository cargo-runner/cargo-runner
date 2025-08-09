#!/bin/bash

# Script to recursively remove all .cargo-runner.json files
# Usage: ./remove-cargo-runner-configs.sh [directory]
# If no directory is specified, starts from current directory

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Starting directory (default to current directory)
START_DIR="${1:-.}"

# Counter for removed files
REMOVED_COUNT=0

echo -e "${YELLOW}Searching for .cargo-runner.json files in: ${START_DIR}${NC}"
echo ""

# Find and list all .cargo-runner.json files first
FILES=$(find "$START_DIR" -name ".cargo-runner.json" -type f 2>/dev/null || true)

if [ -z "$FILES" ]; then
    echo -e "${GREEN}No .cargo-runner.json files found.${NC}"
    exit 0
fi

# Count total files
TOTAL_COUNT=$(echo "$FILES" | wc -l | tr -d ' ')

echo -e "${YELLOW}Found ${TOTAL_COUNT} .cargo-runner.json file(s):${NC}"
echo "$FILES"
echo ""

# Ask for confirmation
read -p "Do you want to remove all these files? (y/N) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo -e "${YELLOW}Removing files...${NC}"
    
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            rm -f "$file"
            echo -e "${RED}Removed: ${file}${NC}"
            ((REMOVED_COUNT++))
        fi
    done <<< "$FILES"
    
    echo ""
    echo -e "${GREEN}Successfully removed ${REMOVED_COUNT} file(s).${NC}"
else
    echo -e "${YELLOW}Operation cancelled.${NC}"
fi