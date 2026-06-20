#!/usr/bin/env bash
set -euo pipefail

echo "==> Building agent-muscle release for macOS..."
cargo build --release -p agent-muscle

BINARY="target/release/agent-muscle"
if [[ ! -f "$BINARY" ]]; then
  echo "Build failed: $BINARY not found" >&2
  exit 1
fi

echo "==> Binary: $BINARY"
echo "==> Size: $(stat -f%z "$BINARY") bytes"
echo "==> Build complete: $BINARY"
