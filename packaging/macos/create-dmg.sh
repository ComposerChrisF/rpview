#!/bin/bash
#
# RPView DMG Creator
# Creates a distributable disk image with drag-to-Applications installer
#
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
APP_NAME="RPView"
VERSION=$(grep -m1 'version' "$PROJECT_DIR/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')
DMG_NAME="${APP_NAME}-${VERSION}"
VOLUME_NAME="${APP_NAME} ${VERSION}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Default paths
APP_PATH="$PROJECT_DIR/target/release/${APP_NAME}.app"
OUTPUT_DIR="$PROJECT_DIR/target/release"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --app)
            APP_PATH="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --app PATH     Path to .app bundle (default: target/release/RPView.app)"
            echo "  --output DIR   Output directory for DMG (default: target/release)"
            echo "  -h, --help     Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Verify app exists
if [ ! -d "$APP_PATH" ]; then
    error "App bundle not found at $APP_PATH"
    echo "Run ./packaging/macos/bundle.sh first to create the app bundle."
fi

DMG_PATH="$OUTPUT_DIR/${DMG_NAME}.dmg"
TEMP_DMG="$OUTPUT_DIR/${DMG_NAME}-temp.dmg"

info "Creating DMG for $APP_NAME version $VERSION..."

# Clean up any existing DMG
rm -f "$DMG_PATH" "$TEMP_DMG"

# Create temporary directory for DMG contents
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Copy app to temp directory
info "Copying app bundle..."
cp -R "$APP_PATH" "$TEMP_DIR/"

# Create symbolic link to Applications folder
ln -s /Applications "$TEMP_DIR/Applications"

# Calculate size needed (app size + 10MB buffer)
APP_SIZE=$(du -sm "$APP_PATH" | cut -f1)
DMG_SIZE=$((APP_SIZE + 10))

info "Creating DMG (${DMG_SIZE}MB)..."

# Create the DMG
hdiutil create -srcfolder "$TEMP_DIR" \
    -volname "$VOLUME_NAME" \
    -fs HFS+ \
    -fsargs "-c c=64,a=16,e=16" \
    -format UDRW \
    -size "${DMG_SIZE}m" \
    "$TEMP_DMG"

# Mount the DMG
info "Configuring DMG appearance..."
MOUNT_DIR="/Volumes/$VOLUME_NAME"

# Unmount if already mounted
if [ -d "$MOUNT_DIR" ]; then
    hdiutil detach "$MOUNT_DIR" -quiet || true
fi

hdiutil attach "$TEMP_DMG" -mountpoint "$MOUNT_DIR" -quiet

# Set DMG window appearance using AppleScript
osascript <<EOF
tell application "Finder"
    tell disk "$VOLUME_NAME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set bounds of container window to {400, 100, 900, 400}
        set viewOptions to the icon view options of container window
        set arrangement of viewOptions to not arranged
        set icon size of viewOptions to 80
        set position of item "$APP_NAME.app" of container window to {125, 150}
        set position of item "Applications" of container window to {375, 150}
        close
        open
        update without registering applications
        delay 2
    end tell
end tell
EOF

# Set custom background (optional - uncomment if you have a background image)
# BACKGROUND_FILE="$SCRIPT_DIR/dmg-background.png"
# if [ -f "$BACKGROUND_FILE" ]; then
#     mkdir -p "$MOUNT_DIR/.background"
#     cp "$BACKGROUND_FILE" "$MOUNT_DIR/.background/background.png"
#     # Additional AppleScript to set background would go here
# fi

sync

# Unmount
hdiutil detach "$MOUNT_DIR" -quiet

# Convert to compressed, read-only DMG
info "Compressing DMG..."
hdiutil convert "$TEMP_DMG" \
    -format UDZO \
    -imagekey zlib-level=9 \
    -o "$DMG_PATH"

rm -f "$TEMP_DMG"

# Generate checksum
info "Generating checksum..."
CHECKSUM=$(shasum -a 256 "$DMG_PATH" | cut -d' ' -f1)
echo "$CHECKSUM  $(basename "$DMG_PATH")" > "$DMG_PATH.sha256"

# Final output
FINAL_SIZE=$(du -h "$DMG_PATH" | cut -f1)
echo ""
info "Successfully created: $DMG_PATH ($FINAL_SIZE)"
info "SHA256: $CHECKSUM"
echo ""
echo "To install:"
echo "  1. Open $DMG_PATH"
echo "  2. Drag $APP_NAME to Applications"
echo ""
