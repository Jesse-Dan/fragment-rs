#!/bin/sh
# Install the Fragment Rust CLI.
#
#   curl -fsSL https://raw.githubusercontent.com/Jesse-Dan/fragment-rs/main/scripts/install.sh | sh
#
# Set FRS_REPO to install from a different repository, and FRS_INSTALL_DIR to
# install somewhere other than ~/.local/bin.
set -eu

REPO=${FRS_REPO:-Jesse-Dan/fragment-rs}
DEST=${FRS_INSTALL_DIR:-$HOME/.local/bin}

case "$(uname -s)/$(uname -m)" in
    Linux/x86_64)  asset=frs-linux-x64 ;;
    Darwin/x86_64) asset=frs-macos-x64 ;;
    Darwin/arm64)  asset=frs-macos-arm64 ;;
    *) echo "Unsupported platform: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "https://github.com/$REPO/releases/latest/download/$asset.tar.gz" -o "$tmp/frs.tar.gz"
tar -xzf "$tmp/frs.tar.gz" -C "$tmp"

mkdir -p "$DEST"
mv "$tmp/frs" "$DEST/frs"
chmod +x "$DEST/frs"

# Put the directory on the PATH so `frs` runs from anywhere.
case ":$PATH:" in
    *":$DEST:"*) ;;
    *)
        for profile in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
            [ -f "$profile" ] || continue
            grep -q "$DEST" "$profile" ||
                echo "export PATH=\"$DEST:\$PATH\"" >> "$profile"
            break
        done
        echo "Added $DEST to your PATH. Open a new shell, or run:"
        echo "  export PATH=\"$DEST:\$PATH\""
        ;;
esac

"$DEST/frs" --version
