#!/usr/bin/env bash
set -euo pipefail

# Playbook: Verify AST Safety Guard
# Tests tree-sitter bash AST syntax and destructive pattern rejection.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

echo "==> [Playbook: AST Guard] Running cargo test -p agent-muscle ast_guard::tests..."
cd "$REPO_ROOT"
cargo test -p agent-muscle ast_guard::tests

echo "==> [Playbook: AST Guard] Testing executor rejection..."
cargo test -p agent-muscle executor::tests::run_command_rejects_ast_syntax_error
cargo test -p agent-muscle executor::tests::run_command_rejects_destructive_root_rm

echo "==> [Playbook: AST Guard] Passed cleanly (exit 0)."
