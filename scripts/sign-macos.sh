#!/usr/bin/env bash
set -euo pipefail

BINARY="${1:-target/release/agent-muscle}"

if [[ ! -f "$BINARY" ]]; then
  echo "Binary not found: $BINARY" >&2
  echo "Usage: $0 [path-to-binary]" >&2
  exit 1
fi

echo "==> Ad-hoc signing: $BINARY"
xattr -cr "$BINARY"
codesign --force --sign - "$BINARY"
codesign --verify --verbose "$BINARY"
echo "==> Signing complete"
