#!/usr/bin/env bash
set -euo pipefail

# Playbook: Verify Output Slicer
# Tests adaptive output compression, failure slicing, and trace preservation.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

echo "==> [Playbook: Output Slicer] Running cargo test -p agent-muscle output_slicer::tests..."
cd "$REPO_ROOT"
cargo test -p agent-muscle output_slicer::tests

echo "==> [Playbook: Output Slicer] Passed cleanly (exit 0)."
