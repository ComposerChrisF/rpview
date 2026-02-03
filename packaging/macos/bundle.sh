#!/bin/bash
#
# RPView macOS App Bundle Creator
# Creates RPView.app from the compiled binary
#
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
APP_NAME="RPView"
BUNDLE_NAME="${APP_NAME}.app"
BINARY_NAME="rpview"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Parse arguments
BUILD=true
SIGN=false
NOTARIZE=false
UNIVERSAL=false
INSTALL=false
OUTPUT_DIR="$PROJECT_DIR/target/release"

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            BUILD=false
            shift
            ;;
        --install)
            INSTALL=true
            shift
            ;;
        --sign)
            SIGN=true
            shift
            ;;
        --notarize)
            NOTARIZE=true
            SIGN=true  # Notarization requires signing
            shift
            ;;
        --universal)
            UNIVERSAL=true
            shift
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --install     Install to /Applications after building"
            echo "  --no-build    Skip cargo build (use existing binary)"
            echo "  --sign        Code sign the app (requires DEVELOPER_ID env var)"
            echo "  --notarize    Notarize the app (requires --sign and Apple credentials)"
            echo "  --universal   Build universal binary (Intel + Apple Silicon)"
            echo "  --output DIR  Output directory (default: target/release)"
            echo "  -h, --help    Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

cd "$PROJECT_DIR"

# Build the binary
if [ "$BUILD" = true ]; then
    if [ "$UNIVERSAL" = true ]; then
        info "Building universal binary (Intel + Apple Silicon)..."

        # Build for Intel
        info "Building for x86_64-apple-darwin..."
        cargo build --release --target x86_64-apple-darwin

        # Build for Apple Silicon
        info "Building for aarch64-apple-darwin..."
        cargo build --release --target aarch64-apple-darwin

        # Create universal binary
        info "Creating universal binary with lipo..."
        mkdir -p "$OUTPUT_DIR"
        lipo -create \
            "target/x86_64-apple-darwin/release/$BINARY_NAME" \
            "target/aarch64-apple-darwin/release/$BINARY_NAME" \
            -output "$OUTPUT_DIR/$BINARY_NAME"
    else
        info "Building release binary..."
        cargo build --release
    fi
fi

# Verify binary exists
BINARY_PATH="$OUTPUT_DIR/$BINARY_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    error "Binary not found at $BINARY_PATH. Run without --no-build first."
fi

# Create app bundle structure
BUNDLE_PATH="$OUTPUT_DIR/$BUNDLE_NAME"
CONTENTS_DIR="$BUNDLE_PATH/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

info "Creating app bundle at $BUNDLE_PATH..."
rm -rf "$BUNDLE_PATH"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy binary
info "Copying binary..."
cp "$BINARY_PATH" "$MACOS_DIR/$BINARY_NAME"
chmod +x "$MACOS_DIR/$BINARY_NAME"

# Copy Info.plist
info "Copying Info.plist..."
if [ -f "$SCRIPT_DIR/Info.plist" ]; then
    cp "$SCRIPT_DIR/Info.plist" "$CONTENTS_DIR/Info.plist"
else
    error "Info.plist not found at $SCRIPT_DIR/Info.plist"
fi

# Copy icon if it exists
if [ -f "$SCRIPT_DIR/rpview.icns" ]; then
    info "Copying icon..."
    cp "$SCRIPT_DIR/rpview.icns" "$RESOURCES_DIR/rpview.icns"
else
    warn "Icon not found at $SCRIPT_DIR/rpview.icns"
    warn "The app will use a generic icon. See packaging/ICONS.md for creation instructions."
fi

# Code signing
if [ "$SIGN" = true ]; then
    if [ -z "$DEVELOPER_ID" ]; then
        # Try to find a valid signing identity
        DEVELOPER_ID=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | sed 's/.*"\(.*\)".*/\1/' || true)
    fi

    if [ -z "$DEVELOPER_ID" ]; then
        warn "No Developer ID found. Skipping code signing."
        warn "Set DEVELOPER_ID environment variable or install a Developer ID certificate."
    else
        info "Signing app with: $DEVELOPER_ID"

        ENTITLEMENTS="$SCRIPT_DIR/rpview.entitlements"
        if [ -f "$ENTITLEMENTS" ]; then
            codesign --force --options runtime --sign "$DEVELOPER_ID" \
                --entitlements "$ENTITLEMENTS" \
                --timestamp \
                "$BUNDLE_PATH"
        else
            codesign --force --options runtime --sign "$DEVELOPER_ID" \
                --timestamp \
                "$BUNDLE_PATH"
        fi

        info "Verifying signature..."
        codesign --verify --deep --strict "$BUNDLE_PATH"
        info "Signature verified successfully."
    fi
fi

# Notarization
if [ "$NOTARIZE" = true ]; then
    if [ -z "$APPLE_ID" ] || [ -z "$APPLE_TEAM_ID" ]; then
        warn "Skipping notarization. Set APPLE_ID and APPLE_TEAM_ID environment variables."
        warn "You also need an app-specific password in your keychain as 'notarytool-password'"
    else
        info "Creating ZIP for notarization..."
        ZIP_PATH="$OUTPUT_DIR/${APP_NAME}.zip"
        ditto -c -k --keepParent "$BUNDLE_PATH" "$ZIP_PATH"

        info "Submitting for notarization..."
        xcrun notarytool submit "$ZIP_PATH" \
            --apple-id "$APPLE_ID" \
            --team-id "$APPLE_TEAM_ID" \
            --keychain-profile "notarytool-password" \
            --wait

        info "Stapling notarization ticket..."
        xcrun stapler staple "$BUNDLE_PATH"

        rm "$ZIP_PATH"
        info "Notarization complete."
    fi
fi

# Install to /Applications
if [ "$INSTALL" = true ]; then
    INSTALL_PATH="/Applications/${BUNDLE_NAME}"
    info "Installing to $INSTALL_PATH..."

    # Remove existing installation
    if [ -d "$INSTALL_PATH" ]; then
        rm -rf "$INSTALL_PATH"
    fi

    cp -R "$BUNDLE_PATH" /Applications/

    # Remove quarantine attribute if not signed
    if [ "$SIGN" != true ]; then
        xattr -cr "$INSTALL_PATH"
    fi

    echo ""
    info "Successfully installed: $INSTALL_PATH"
    echo ""
    echo "To run the app:"
    echo "  open -a RPView"
    echo ""
else
    # Final output
    echo ""
    info "Successfully created: $BUNDLE_PATH"
    echo ""
    echo "To run the app:"
    echo "  open \"$BUNDLE_PATH\""
    echo ""
    echo "To install to /Applications:"
    echo "  $0 --no-build --install"
    echo ""

    if [ "$SIGN" != true ]; then
        echo "Note: The app is not code signed. To run unsigned apps:"
        echo "  xattr -cr \"$BUNDLE_PATH\""
        echo "  # Or right-click the app and select 'Open' to bypass Gatekeeper"
        echo ""
    fi
fi
