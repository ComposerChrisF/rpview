#!/bin/bash
# Installation script for RPView on Linux

set -e

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
DESKTOP_DIR="${DESKTOP_DIR:-$HOME/.local/share/applications}"
ICON_DIR="${ICON_DIR:-$HOME/.local/share/icons/hicolor}"

echo "Installing RPView..."

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR"
mkdir -p "$DESKTOP_DIR"

# Copy binary
if [ -f "../../target/release/rpview" ]; then
    cp "../../target/release/rpview" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/rpview"
    echo "Binary installed to $INSTALL_DIR/rpview"
else
    echo "Error: Binary not found at ../../target/release/rpview"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

# Copy desktop file
cp rpview.desktop "$DESKTOP_DIR/"
echo "Desktop file installed to $DESKTOP_DIR/rpview.desktop"

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR"
    echo "Desktop database updated"
fi

# Update MIME database
if command -v update-mime-database >/dev/null 2>&1; then
    update-mime-database "$HOME/.local/share/mime"
    echo "MIME database updated"
fi

echo ""
echo "RPView has been installed successfully!"
echo "You can now run 'rpview' from the command line"
echo "or find it in your application menu."
echo ""
echo "To uninstall, run: rm $INSTALL_DIR/rpview $DESKTOP_DIR/rpview.desktop"
