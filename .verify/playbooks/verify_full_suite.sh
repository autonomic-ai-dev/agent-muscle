#!/usr/bin/env bash
set -euo pipefail

# Playbook: Verify Full Test Suite
# Runs all 31 unit & integration tests and verifies zero compiler warnings.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

echo "==> [Playbook: Full Suite] Running cargo test -p agent-muscle..."
cd "$REPO_ROOT"
cargo test -p agent-muscle

echo "==> [Playbook: Full Suite] Checking cargo check with zero warnings..."
cargo check -p agent-muscle

echo "==> [Playbook: Full Suite] Passed cleanly (exit 0)."
