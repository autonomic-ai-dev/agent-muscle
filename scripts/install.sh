#!/usr/bin/env bash
set -euo pipefail

REPO="autonomic-ai-dev/agent-muscle"
BINARY="agent-muscle"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect platform and architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux)
    case "$ARCH" in
      x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  darwin)
    case "$ARCH" in
      x86_64) TARGET="x86_64-apple-darwin" ;;
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

ARTIFACT="${BINARY}-${TARGET}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"

echo "==> Installing agent-muscle (${TARGET})"
echo "==> Downloading from ${DOWNLOAD_URL}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$BINARY"
chmod +x "$TMP_DIR/$BINARY"

if [[ ! -w "$INSTALL_DIR" ]]; then
  echo "==> Need sudo to install to ${INSTALL_DIR}"
  sudo mv "$TMP_DIR/$BINARY" "${INSTALL_DIR}/${BINARY}"
else
  mv "$TMP_DIR/$BINARY" "${INSTALL_DIR}/${BINARY}"
fi

# macOS Gatekeeper: clear quarantine + ad-hoc sign
if [[ "$(uname -s)" == "Darwin" ]] && command -v codesign >/dev/null 2>&1; then
  xattr -cr "${INSTALL_DIR}/${BINARY}" 2>/dev/null || true
  codesign --force --sign - "${INSTALL_DIR}/${BINARY}" 2>/dev/null || true
  echo "macOS: cleared quarantine and adhoc-signed ${BINARY}"
fi

echo "==> Installed: ${INSTALL_DIR}/${BINARY}"
echo "==> Run 'agent-muscle --help' to get started"
