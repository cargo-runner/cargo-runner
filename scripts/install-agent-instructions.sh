#!/usr/bin/env bash
# Install / refresh cargo-runner agent instructions into AI config files.
#
# Usage:
#   ./scripts/install-agent-instructions.sh
#       Scan the git repo (or cwd) for common agent files and update each
#       unique real file once. Symlinks are followed and deduped
#       (AGENTS.md → CLAUDE.md updates CLAUDE.md only).
#
#   ./scripts/install-agent-instructions.sh AGENTS.md CLAUDE.md .cursor/rules/foo.mdc
#       Update only these paths (relative to --root / cwd, or absolute).
#
#   ./scripts/install-agent-instructions.sh --root ~/Code/my-app
#   ./scripts/install-agent-instructions.sh --dry-run
#   ./scripts/install-agent-instructions.sh --create-agents
#       Create AGENTS.md when scan finds no files (default when scanning).
#   ./scripts/install-agent-instructions.sh --no-create
#   ./scripts/install-agent-instructions.sh --source /path/to/AGENTS.cargo-runner.md
#
# Managed block markers (idempotent):
#   <!-- BEGIN cargo-runner agent instructions -->
#   ...
#   <!-- END cargo-runner agent instructions -->
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_OF_SCRIPT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEFAULT_SOURCE="$REPO_OF_SCRIPT/docs/AGENTS.cargo-runner.md"

ROOT=""
SOURCE="${CARGO_RUNNER_AGENT_DOC:-$DEFAULT_SOURCE}"
DRY_RUN=0
CREATE_AGENTS=1   # default on for scan mode; flipped off if --no-create
NO_CREATE=0
VERBOSE=0
declare -a EXPLICIT_PATHS=()

usage() {
  sed -n '2,28p' "$0"
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage ;;
    --root)
      ROOT="${2:?--root needs a directory}"
      shift 2
      ;;
    --source)
      SOURCE="${2:?--source needs a file}"
      shift 2
      ;;
    --dry-run) DRY_RUN=1; shift ;;
    --create-agents) CREATE_AGENTS=1; NO_CREATE=0; shift ;;
    --no-create) NO_CREATE=1; CREATE_AGENTS=0; shift ;;
    -v|--verbose) VERBOSE=1; shift ;;
    --) shift; EXPLICIT_PATHS+=("$@"); break ;;
    -*)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
    *)
      EXPLICIT_PATHS+=("$1")
      shift
      ;;
  esac
done

if [[ -z "$ROOT" ]]; then
  if git rev-parse --show-toplevel >/dev/null 2>&1; then
    ROOT="$(git rev-parse --show-toplevel)"
  else
    ROOT="$(pwd)"
  fi
fi
ROOT="$(cd "$ROOT" && pwd)"

if [[ ! -f "$SOURCE" ]]; then
  echo "error: instruction source not found: $SOURCE" >&2
  echo "  Set CARGO_RUNNER_AGENT_DOC or pass --source PATH" >&2
  exit 1
fi
SOURCE="$(cd "$(dirname "$SOURCE")" && pwd)/$(basename "$SOURCE")"

EXPLICIT=0
if [[ ${#EXPLICIT_PATHS[@]} -gt 0 ]]; then
  EXPLICIT=1
  # Explicit paths: only create those that are missing when not --no-create
  if [[ "$NO_CREATE" -eq 0 ]]; then
    CREATE_AGENTS=1
  fi
fi

echo "Project root : $ROOT"
echo "Source       : $SOURCE"
[[ "$DRY_RUN" -eq 1 ]] && echo "Mode         : dry-run"

# Build candidate list
CAND_FILE="$(mktemp)"
trap 'rm -f "$CAND_FILE"' EXIT

if [[ "$EXPLICIT" -eq 1 ]]; then
  for p in "${EXPLICIT_PATHS[@]}"; do
    if [[ "$p" = /* ]]; then
      printf '%s\0' "$p" >>"$CAND_FILE"
    else
      printf '%s\0' "$ROOT/$p" >>"$CAND_FILE"
    fi
  done
else
  COMMON=(
    "AGENTS.md"
    "CLAUDE.md"
    "GEMINI.md"
    "AGENT.md"
    ".cursorrules"
    ".windsurfrules"
    ".github/copilot-instructions.md"
    ".github/instructions/cargo-runner.instructions.md"
  )
  for name in "${COMMON[@]}"; do
    printf '%s\0' "$ROOT/$name" >>"$CAND_FILE"
  done
  if [[ -d "$ROOT/.cursor/rules" ]]; then
    # portable find without -print0 fail
    find "$ROOT/.cursor/rules" -maxdepth 2 \( -name '*.md' -o -name '*.mdc' -o -name '*.markdown' \) 2>/dev/null \
      | while IFS= read -r f; do printf '%s\0' "$f" >>"$CAND_FILE"; done || true
  fi
  if [[ -d "$ROOT/.claude" ]]; then
    find "$ROOT/.claude" -maxdepth 2 -name '*.md' 2>/dev/null \
      | while IFS= read -r f; do printf '%s\0' "$f" >>"$CAND_FILE"; done || true
  fi
fi

export CR_ROOT="$ROOT"
export CR_SOURCE="$SOURCE"
export CR_DRY_RUN="$DRY_RUN"
export CR_CREATE_AGENTS="$CREATE_AGENTS"
export CR_EXPLICIT="$EXPLICIT"
export CR_VERBOSE="$VERBOSE"
export CR_CAND_FILE="$CAND_FILE"

python3 <<'PY'
import os, sys, re
from pathlib import Path

ROOT = Path(os.environ["CR_ROOT"]).resolve()
SOURCE = Path(os.environ["CR_SOURCE"]).resolve()
DRY = os.environ.get("CR_DRY_RUN") == "1"
CREATE = os.environ.get("CR_CREATE_AGENTS") == "1"
EXPLICIT = os.environ.get("CR_EXPLICIT") == "1"
VERBOSE = os.environ.get("CR_VERBOSE") == "1"

BEGIN = "<!-- BEGIN cargo-runner agent instructions -->"
END = "<!-- END cargo-runner agent instructions -->"

def dbg(*a):
    if VERBOSE:
        print("  ·", *a, file=sys.stderr)

def rel(p: Path) -> str:
    try:
        return str(p.resolve().relative_to(ROOT))
    except Exception:
        try:
            return str(p.relative_to(ROOT))
        except Exception:
            return str(p)

raw = SOURCE.read_text(encoding="utf-8")
if "## Golden rule" in raw:
    title = "# cargo-runner — agent instructions\n\n"
    body = title + raw[raw.index("## Golden rule") :]
else:
    body = raw
body = body.strip() + "\n"
block = f"{BEGIN}\n\n{body}\n{END}\n"

# Load candidates
raw_bytes = Path(os.environ["CR_CAND_FILE"]).read_bytes()
candidates = [
    Path(os.fsdecode(p))
    for p in raw_bytes.split(b"\0")
    if p
]

# real path -> list of alias paths (for display)
by_real: dict[str, list[Path]] = {}
missing_explicit: list[Path] = []

for c in candidates:
    # Broken symlink
    if c.is_symlink() and not c.exists():
        dbg(f"skip broken symlink: {c}")
        continue
    if not c.exists():
        if EXPLICIT:
            missing_explicit.append(c)
        continue
    if c.is_dir():
        dbg(f"skip directory: {c}")
        continue
    try:
        real = c.resolve()
    except Exception as e:
        dbg(f"resolve failed {c}: {e}")
        continue
    if not real.is_file() and not real.exists():
        continue
    # If resolve points at a dir, skip
    if real.is_dir():
        continue
    by_real.setdefault(str(real), []).append(c)

# Create missing explicit paths as normal files (do not invent symlinks)
if EXPLICIT and CREATE:
    for c in missing_explicit:
        # Write key as the path itself (new file)
        key = str(c if c.is_absolute() else (ROOT / c))
        # normalize
        p = Path(key)
        by_real.setdefault(str(p), []).append(p)
        dbg(f"will create: {p}")

# Scan mode: nothing found → optional AGENTS.md
if not by_real and not EXPLICIT and CREATE:
    agents = ROOT / "AGENTS.md"
    by_real[str(agents)] = [agents]
    print(f"No agent files found; will create {rel(agents)}")

if not by_real:
    print("Nothing to update.")
    sys.exit(1)

updated = skipped = 0

for real_s in sorted(by_real.keys()):
    real = Path(real_s)
    aliases = by_real[real_s]

    # Display names + symlink notes
    names = []
    link_bits = []
    for a in aliases:
        names.append(rel(a) if str(a).startswith(str(ROOT)) else str(a))
        if a.is_symlink():
            try:
                link_bits.append(f"{a.name}→{os.readlink(a)}")
            except OSError:
                link_bits.append(f"{a.name}→?")
    display = ", ".join(sorted(set(names)))
    link_note = f" (symlinks: {', '.join(link_bits)})" if link_bits else ""

    # Always write the resolved real file (not the symlink path as a new file)
    # If path does not exist yet, create at `real` (for explicit create, real may be intended path)
    write_path = real
    if not write_path.exists() and aliases:
        # Prefer non-symlink alias path for creation
        for a in aliases:
            if not a.is_symlink():
                write_path = a
                break
        else:
            write_path = aliases[0]

    if write_path.exists() and write_path.is_symlink():
        write_path = write_path.resolve()

    if write_path.exists():
        text = write_path.read_text(encoding="utf-8")
        if BEGIN in text and END in text:
            pattern = re.compile(
                re.escape(BEGIN) + r".*?" + re.escape(END) + r"\n?",
                re.DOTALL,
            )
            new_text = pattern.sub(lambda _m: block.rstrip("\n") + "\n", text, count=1)
            action = "unchanged" if new_text == text else "update"
        else:
            if text and not text.endswith("\n"):
                text += "\n"
            new_text = text + ("\n" if text.strip() else "") + block
            action = "append"
    else:
        new_text = block
        action = "create"

    if action == "unchanged":
        print(f"= skip (already current): {display}{link_note}")
        skipped += 1
        continue

    if DRY:
        print(f"~ would {action}: {display}{link_note}")
        print(f"    → {write_path}")
        updated += 1
        continue

    write_path.parent.mkdir(parents=True, exist_ok=True)
    # Refuse to replace a symlink path with a regular file if caller passed the symlink
    # (we already resolved write_path to the target)
    write_path.write_text(new_text, encoding="utf-8")
    print(f"✓ {action}: {display}{link_note}")
    dbg(f"wrote {write_path} ({len(new_text)} bytes)")
    updated += 1

print()
print(f"Done. updated={updated} skipped={skipped} unique_targets={len(by_real)}")
if DRY:
    print("(dry-run: no files written)")
sys.exit(0)
PY
