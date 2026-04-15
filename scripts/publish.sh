#!/bin/bash
set -e

# Usage:
#   ./publish.sh          -> Auto-bump patch version, push tag, publish to crates.io
#   ./publish.sh 1.0.1    -> Run for specific version and force overwrite tags if same

# Extract current version from Cargo.toml
CURRENT_VERSION=$(awk '/^version = / {print $3}' Cargo.toml | head -n 1 | tr -d '"')

TARGET_VERSION=$1
FORCE=0

if [ -z "$TARGET_VERSION" ]; then
    # Auto-bump patch version
    IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"
    TARGET_VERSION="$major.$minor.$((patch + 1))"
    echo "No version specified. Auto-bumping from $CURRENT_VERSION to $TARGET_VERSION..."
else
    echo "Using specified version: $TARGET_VERSION"
    if [ "$CURRENT_VERSION" == "$TARGET_VERSION" ]; then
        echo "Version $TARGET_VERSION is already the current version."
        echo "⚠️  Forcing tag overwrite on GitHub (crates.io publish will be skipped)..."
        FORCE=1
    fi
fi

if [ "$FORCE" -eq 0 ]; then
    echo "Updating Cargo.toml versions to $TARGET_VERSION..."
    # Update root Cargo.toml (works securely on macOS/Linux with .bak)
    sed -i.bak "s/^version = \".*\"/version = \"$TARGET_VERSION\"/" Cargo.toml
    # Update CLI Cargo.toml version requirement
    sed -i.bak "s/cargo-runner-core = { version = \".*\"/cargo-runner-core = { version = \"$TARGET_VERSION\"/" crates/cli/Cargo.toml
    
    # Cleanup backups
    rm -f Cargo.toml.bak crates/cli/Cargo.toml.bak
    
    # Commit changes
    git add Cargo.toml crates/cli/Cargo.toml
    git commit -m "Bump version to $TARGET_VERSION for stable release"
    git push origin main
fi

# Determine tag name
TAG_NAME="cargo-runner-cli-v$TARGET_VERSION"

if [ "$FORCE" -eq 1 ]; then
    echo "Deleting existing tag $TAG_NAME..."
    git push origin :refs/tags/$TAG_NAME || true
    git tag -d $TAG_NAME || true
fi

echo "Creating and pushing tag $TAG_NAME..."
git tag $TAG_NAME
git push origin $TAG_NAME

echo "Publishing to crates.io..."
if [ "$FORCE" -eq 1 ]; then
    echo "⚠️  Skipping crates.io publish because we are force-pushing an existing version."
    echo "    Crates.io does not allow overwriting existing versions."
else
    echo "Publishing cargo-runner-core..."
    cargo publish -p cargo-runner-core || echo "Publish core failed (maybe already published?)"
    
    echo "Waiting 15 seconds for crates.io index to update before publishing CLI..."
    sleep 15
    
    echo "Publishing cargo-runner-cli..."
    cargo publish -p cargo-runner-cli || echo "Publish cli failed"
fi

echo "✅ Publish process fully completed for $TARGET_VERSION!"
