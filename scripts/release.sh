#!/usr/bin/env bash
# cargo-runner release helper
#
# Bump policy (see docs/release.md):
#   vscode / product  → minor  (1.6.3 → 1.7.0)  when extension or product surface changes
#   cli               → patch  (1.6.3 → 1.6.4)  when only CLI/core changes
#   major             → major  (1.6.3 → 2.0.0)  breaking
#
# Usage:
#   ./scripts/release.sh cli
#   ./scripts/release.sh vscode
#   ./scripts/release.sh major
#   ./scripts/release.sh 1.7.0
#   ./scripts/release.sh cli --dry-run
#   ./scripts/release.sh vscode --marketplace
#   ./scripts/release.sh cli --no-crates --no-push
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MODE="${1:-}"
shift || true

DRY_RUN=0
NO_CRATES=0
NO_PUSH=0
MARKETPLACE=0
FORCE_TAG=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --no-crates) NO_CRATES=1 ;;
    --no-push) NO_PUSH=1 ;;
    --marketplace) MARKETPLACE=1 ;;
    --force-tag) FORCE_TAG=1 ;; # re-tag same version (crates.io skipped if already current)
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
  shift
done

if [[ -z "$MODE" ]]; then
  echo "Usage: $0 {cli|vscode|major|X.Y.Z} [--dry-run] [--no-crates] [--no-push] [--marketplace] [--force-tag]" >&2
  echo "See docs/release.md" >&2
  exit 1
fi

current_version() {
  awk '/^version = / {print $3; exit}' Cargo.toml | tr -d '"'
}

bump_semver() {
  local ver="$1" kind="$2"
  local major minor patch
  IFS='.' read -r major minor patch <<<"$ver"
  major=${major:-0}
  minor=${minor:-0}
  patch=${patch:-0}
  case "$kind" in
    major) echo "$((major + 1)).0.0" ;;
    minor) echo "${major}.$((minor + 1)).0" ;;
    patch) echo "${major}.${minor}.$((patch + 1))" ;;
    *) echo "bad kind $kind" >&2; exit 1 ;;
  esac
}

CURRENT="$(current_version)"
if [[ ! "$CURRENT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Could not parse current version from Cargo.toml: '$CURRENT'" >&2
  exit 1
fi

case "$MODE" in
  cli|patch)
    KIND=patch
    TARGET="$(bump_semver "$CURRENT" patch)"
    REASON="CLI/core-only changes (patch)"
    ;;
  vscode|extension|product|minor)
    KIND=minor
    TARGET="$(bump_semver "$CURRENT" minor)"
    REASON="VS Code / product surface changes (minor)"
    ;;
  major)
    KIND=major
    TARGET="$(bump_semver "$CURRENT" major)"
    REASON="Breaking changes (major)"
    ;;
  [0-9]*.[0-9]*.[0-9]*)
    KIND=explicit
    TARGET="$MODE"
    REASON="Explicit version"
    ;;
  *)
    echo "Unknown mode: $MODE (use cli|vscode|major|X.Y.Z)" >&2
    exit 1
    ;;
esac

TAG_NAME="cargo-runner-cli-v${TARGET}"

echo "══════════════════════════════════════════════"
echo " cargo-runner release"
echo "══════════════════════════════════════════════"
echo " Current : $CURRENT"
echo " Target  : $TARGET"
echo " Reason  : $REASON"
echo " Tag     : $TAG_NAME"
echo " crates  : $([ "$NO_CRATES" -eq 1 ] && echo skip || echo yes)"
echo " push    : $([ "$NO_PUSH" -eq 1 ] && echo skip || echo yes)"
echo " market  : $([ "$MARKETPLACE" -eq 1 ] && echo yes || echo no)"
echo " dry-run : $([ "$DRY_RUN" -eq 1 ] && echo yes || echo no)"
echo "══════════════════════════════════════════════"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "(dry-run) No files changed."
  exit 0
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is not clean. Commit or stash first." >&2
  git status -sb
  exit 1
fi

if [[ "$CURRENT" == "$TARGET" ]]; then
  if [[ "$FORCE_TAG" -eq 1 ]]; then
    echo "Version already $TARGET — re-tagging GitHub release only (crates.io cannot overwrite)."
    NO_CRATES=1
  else
    echo "Already at $TARGET. Use a bump mode or --force-tag to rebuild GitHub assets." >&2
    exit 1
  fi
else
  echo "→ Bumping version files to $TARGET"
  # workspace version
  sed -i.bak "s/^version = \".*\"/version = \"$TARGET\"/" Cargo.toml
  # path dependency pin in CLI
  sed -i.bak "s/cargo-runner-core = { version = \"[^\"]*\"/cargo-runner-core = { version = \"$TARGET\"/" crates/cli/Cargo.toml
  # VS Code extension (kept in lockstep for Download CLI tag)
  if [[ -f extensions/vscode/package.json ]]; then
    # only replace top-level "version" field (first occurrence after name/description block)
    python3 - <<PY
import json, pathlib
p = pathlib.Path("extensions/vscode/package.json")
data = json.loads(p.read_text())
data["version"] = "$TARGET"
p.write_text(json.dumps(data, indent=2) + "\n")
PY
  fi
  rm -f Cargo.toml.bak crates/cli/Cargo.toml.bak
  cargo update -p cargo-runner-cli -p cargo-runner-core 2>/dev/null || true

  # CHANGELOG stub if missing
  if [[ -f CHANGELOG.md ]] && ! grep -q "## \[${TARGET}\]" CHANGELOG.md; then
    DATE="$(date -u +%Y-%m-%d)"
    stub=$(cat <<EOF
## [${TARGET}] — ${DATE}

### Changed

- Release (${REASON}).

EOF
)
    # Insert after the first "---" following the header, or after line 11
    python3 - <<PY
from pathlib import Path
p = Path("CHANGELOG.md")
text = p.read_text()
stub = """$stub"""
marker = "---\n\n## ["
if marker in text:
    text = text.replace(marker, "---\n\n" + stub + "## [", 1)
else:
    text = stub + "\n" + text
p.write_text(text)
print("CHANGELOG.md: added stub for $TARGET")
PY
  fi

  git add Cargo.toml Cargo.lock crates/cli/Cargo.toml extensions/vscode/package.json CHANGELOG.md 2>/dev/null || true
  git add Cargo.toml crates/cli/Cargo.toml extensions/vscode/package.json 2>/dev/null || true
  git commit -m "Bump version to ${TARGET} (${REASON})"
  if [[ "$NO_PUSH" -eq 0 ]]; then
    git push origin HEAD
  fi
fi

# Tag
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
  if [[ "$FORCE_TAG" -eq 1 ]]; then
    echo "→ Moving tag $TAG_NAME to HEAD"
    if [[ "$NO_PUSH" -eq 0 ]]; then
      gh release delete "$TAG_NAME" --yes 2>/dev/null || true
      git push origin ":refs/tags/${TAG_NAME}" 2>/dev/null || true
    fi
    git tag -d "$TAG_NAME" 2>/dev/null || true
  else
    echo "Tag $TAG_NAME already exists. Use --force-tag to recreate." >&2
    exit 1
  fi
fi

echo "→ Creating tag $TAG_NAME"
git tag -a "$TAG_NAME" -m "Release cargo-runner-cli ${TARGET}"
if [[ "$NO_PUSH" -eq 0 ]]; then
  git push origin "$TAG_NAME"
  echo "→ GitHub Actions will build multi-arch assets for $TAG_NAME"
fi

# crates.io
if [[ "$NO_CRATES" -eq 0 ]]; then
  echo "→ Publishing crates.io (core then cli)"
  cargo publish -p cargo-runner-core --allow-dirty 2>&1 || {
    echo "warn: cargo-runner-core publish failed (already published?)" >&2
  }
  echo "   waiting for index..."
  sleep 20
  cargo publish -p cargo-runner-cli --allow-dirty 2>&1 || {
    echo "warn: cargo-runner-cli publish failed (already published?)" >&2
  }
else
  echo "→ Skipping crates.io"
fi

# Marketplace
if [[ "$MARKETPLACE" -eq 1 ]]; then
  echo "→ Publishing VS Code extension (vsce)"
  if ! command -v vsce >/dev/null 2>&1 && ! command -v npx >/dev/null 2>&1; then
    echo "vsce/npx not found" >&2
    exit 1
  fi
  (
    cd extensions/vscode
    if command -v vsce >/dev/null 2>&1; then
      vsce publish --skip-license
    else
      npx --yes @vscode/vsce publish --skip-license
    fi
  )
else
  echo "→ Skipping Marketplace (pass --marketplace for vscode product releases)"
fi

echo ""
echo "✅ Release flow finished for $TARGET"
echo "   Tag:     $TAG_NAME"
echo "   Docs:    docs/release.md"
echo "   Watch:   gh run list --workflow=release.yml --limit 1"
