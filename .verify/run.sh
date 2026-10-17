#!/usr/bin/env bash
set -euo pipefail

# agent-muscle verification entrypoint
# Executed by `agent-brain verify -- .verify/run.sh`

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/.." && pwd)"

echo "==> [agent-muscle] Starting verification suite..."
cd "$REPO_ROOT"

echo "==> Running cargo test -p agent-muscle..."
cargo test -p agent-muscle

echo "==> Checking cargo check -p agent-muscle..."
cargo check -p agent-muscle

echo "==> [agent-muscle] Verification passed cleanly (exit 0)."
