#!/usr/bin/env bash
# Deprecated entrypoint — use scripts/release.sh (see docs/release.md).
#
# Legacy:
#   ./scripts/publish.sh          → patch (CLI)
#   ./scripts/publish.sh 1.7.0    → explicit version
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ $# -eq 0 ]]; then
  exec bash "$ROOT/scripts/release.sh" cli
fi
if [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  exec bash "$ROOT/scripts/release.sh" "$1" "${@:2}"
fi
exec bash "$ROOT/scripts/release.sh" "$@"