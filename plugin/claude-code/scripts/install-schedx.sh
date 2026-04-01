#!/bin/sh
# schedx installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh
set -eu

REPO="${SCHEDX_REPO:-Alireza29675/schedx}"
INSTALL_DIR="${SCHEDX_INSTALL_DIR:-$HOME/.local/bin}"

# Detect architecture
detect_target() {
    arch=$(uname -m)
    os=$(uname -s)

    case "$os" in
        Linux)
            case "$arch" in
                x86_64|amd64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                *)
                    echo "Error: Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64|amd64) echo "x86_64-apple-darwin" ;;
                arm64|aarch64) echo "aarch64-apple-darwin" ;;
                *)
                    echo "Error: Unsupported architecture: $arch"
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Error: Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

# Find sha256sum tool
sha256_cmd() {
    if command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        echo "shasum -a 256"
    else
        echo "Error: Neither sha256sum nor shasum found."
        exit 1
    fi
}

main() {
    target=$(detect_target)
    sha_tool=$(sha256_cmd)

    # Get latest release tag
    if [ -n "${SCHEDX_VERSION:-}" ]; then
        version="$SCHEDX_VERSION"
    else
        version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | grep '"tag_name"' | head -1 | cut -d'"' -f4)
    fi

    if [ -z "$version" ]; then
        echo "Error: Could not determine latest version."
        exit 1
    fi

    echo "Installing schedx $version for $target..."

    tarball="schedx-${target}.tar.gz"
    base_url="https://github.com/$REPO/releases/download/$version"

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    # Download tarball and checksums
    echo "Downloading $tarball..."
    curl -fsSL "$base_url/$tarball" -o "$tmpdir/$tarball"
    curl -fsSL "$base_url/checksums.txt" -o "$tmpdir/checksums.txt"

    # Verify checksum
    echo "Verifying checksum..."
    cd "$tmpdir"
    expected=$(grep "$tarball" checksums.txt | awk '{print $1}')
    if [ -z "$expected" ]; then
        echo "Error: Checksum not found for $tarball in checksums.txt"
        exit 1
    fi

    actual=$($sha_tool "$tarball" | awk '{print $1}')
    if [ "$expected" != "$actual" ]; then
        echo "Error: Checksum mismatch!"
        echo "  Expected: $expected"
        echo "  Actual:   $actual"
        echo "Aborting installation."
        exit 1
    fi
    echo "Checksum verified."

    # Extract and install
    tar xzf "$tarball"
    mkdir -p "$INSTALL_DIR"
    mv schedx "$INSTALL_DIR/schedx"
    chmod +x "$INSTALL_DIR/schedx"

    echo ""
    echo "Installed schedx to $INSTALL_DIR/schedx"

    # PATH hint
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo ""
            echo "Add to your PATH if not already present:"
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac

    echo ""
    echo "Run 'schedx --help' to get started."

    # Offer to set up agent skills (only in interactive terminals)
    if [ -t 0 ]; then
        echo ""
        echo "Would you like to set up schedx for your AI coding agents?"
        echo "This teaches Claude Code, Codex, Cursor, and others how to use schedx."
        printf "Run setup? [Y/n] "
        read -r response
        case "$response" in
            [nN]*) echo "Skipped. Run 'schedx setup' anytime." ;;
            *) "$INSTALL_DIR/schedx" setup ;;
        esac
    else
        echo ""
        echo "Run 'schedx setup' to install skills for your AI coding agents."
    fi
}

main
