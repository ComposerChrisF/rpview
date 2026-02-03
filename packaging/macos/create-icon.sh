#!/bin/bash
#
# RPView Icon Creator for macOS
# Converts a source PNG to .icns format
#
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Check for input file
SOURCE_PNG="$1"

if [ -z "$SOURCE_PNG" ]; then
    echo "Usage: $0 <source-image.png>"
    echo ""
    echo "Creates rpview.icns from a source PNG image."
    echo "The source image should be at least 1024x1024 pixels."
    echo ""
    echo "Example:"
    echo "  $0 ~/Downloads/my-icon.png"
    exit 1
fi

if [ ! -f "$SOURCE_PNG" ]; then
    error "File not found: $SOURCE_PNG"
fi

# Check image dimensions
if command -v sips &> /dev/null; then
    WIDTH=$(sips -g pixelWidth "$SOURCE_PNG" | tail -1 | awk '{print $2}')
    HEIGHT=$(sips -g pixelHeight "$SOURCE_PNG" | tail -1 | awk '{print $2}')

    if [ "$WIDTH" -lt 1024 ] || [ "$HEIGHT" -lt 1024 ]; then
        warn "Source image is ${WIDTH}x${HEIGHT}. Recommended size is 1024x1024 or larger."
    fi
fi

# Create iconset directory
ICONSET_DIR="$SCRIPT_DIR/rpview.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

info "Creating icon sizes from $SOURCE_PNG..."

# Generate all required sizes
sips -z 16 16     "$SOURCE_PNG" --out "$ICONSET_DIR/icon_16x16.png"      >/dev/null
sips -z 32 32     "$SOURCE_PNG" --out "$ICONSET_DIR/icon_16x16@2x.png"   >/dev/null
sips -z 32 32     "$SOURCE_PNG" --out "$ICONSET_DIR/icon_32x32.png"      >/dev/null
sips -z 64 64     "$SOURCE_PNG" --out "$ICONSET_DIR/icon_32x32@2x.png"   >/dev/null
sips -z 128 128   "$SOURCE_PNG" --out "$ICONSET_DIR/icon_128x128.png"    >/dev/null
sips -z 256 256   "$SOURCE_PNG" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$SOURCE_PNG" --out "$ICONSET_DIR/icon_256x256.png"    >/dev/null
sips -z 512 512   "$SOURCE_PNG" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$SOURCE_PNG" --out "$ICONSET_DIR/icon_512x512.png"    >/dev/null
sips -z 1024 1024 "$SOURCE_PNG" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

info "Converting to .icns format..."

# Convert to icns
iconutil -c icns "$ICONSET_DIR" -o "$SCRIPT_DIR/rpview.icns"

# Clean up
rm -rf "$ICONSET_DIR"

info "Created: $SCRIPT_DIR/rpview.icns"
echo ""
echo "You can now run ./bundle.sh to create the app bundle with this icon."
