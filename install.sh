#!/usr/bin/env bash
# weavr installer — downloads the latest prebuilt binary from GitHub Releases.
# Usage: curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/deepakkumardewani/weavr/main/install.sh | sh
set -euo pipefail

REPO="deepakkumardewani/weavr"
BIN="weavr"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64) TARGET="aarch64-apple-darwin" ;;
            x86_64)
                echo "Prebuilt Intel macOS binaries are not provided."
                echo "Install with: cargo install weavr   (or run the Apple Silicon build under Rosetta 2)"
                exit 1
                ;;
            *) echo "Unsupported architecture: $ARCH on macOS"; exit 1 ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH on Linux"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Get latest release URL
echo "Fetching latest weavr release..."
RELEASE_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
    | grep "browser_download_url.*$TARGET.tar.gz" \
    | cut -d '"' -f 4 \
    | head -1)

if [ -z "$RELEASE_URL" ]; then
    echo "Error: could not find release for $TARGET"
    exit 1
fi

# Download and install
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading weavr for $TARGET..."
curl -sL "$RELEASE_URL" -o "$TMP_DIR/weavr.tar.gz"
tar xzf "$TMP_DIR/weavr.tar.gz" -C "$TMP_DIR"

# Install to ~/.cargo/bin or /usr/local/bin
if [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="$HOME/.cargo/bin"
elif [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

cp "$TMP_DIR/weavr" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/weavr"

echo ""
echo "weavr installed to $INSTALL_DIR/weavr"
echo "Run 'weavr --help' to get started."
