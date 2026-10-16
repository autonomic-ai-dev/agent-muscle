#!/usr/bin/env bash
# Build agent-muscle release and Developer ID–sign on macOS (Apple Silicon).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo build --release -p agent-muscle
"$ROOT/scripts/sign-macos.sh"
