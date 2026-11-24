#!/usr/bin/env bash
set -euo pipefail

# -----------------------------------------------------------------------------
# Shark AppImage Local Build Script
# -----------------------------------------------------------------------------
# Packages the Shark file manager into a standalone x86_64 AppImage.
#
# Usage:
#   ./packaging/build-appimage.sh [options]
#
# Options:
#   --no-build       Skip `cargo build --release` (uses existing target/release/shark)
#   --debug          Package target/debug/shark instead of release build
#   -o, --output DIR Output directory for the .AppImage (default: dist)
#   -h, --help       Show this help message
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BUILD_RELEASE=true
USE_DEBUG=false
OUTPUT_DIR="$REPO_ROOT/dist"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build)
      BUILD_RELEASE=false
      shift
      ;;
    --debug)
      BUILD_RELEASE=false
      USE_DEBUG=true
      shift
      ;;
    -o|--output)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  --no-build       Skip cargo build (uses existing target/release/shark)"
      echo "  --debug          Use target/debug/shark binary"
      echo "  -o, --output DIR Output directory for .AppImage (default: dist)"
      echo "  -h, --help       Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

cd "$REPO_ROOT"

# 1. Compile Shark binary if requested
if [ "$BUILD_RELEASE" = true ]; then
  echo "==> Building Shark in release mode..."
  cargo build --release
  BINARY="$REPO_ROOT/target/release/shark"
elif [ "$USE_DEBUG" = true ]; then
  BINARY="$REPO_ROOT/target/debug/shark"
else
  BINARY="$REPO_ROOT/target/release/shark"
fi

if [ ! -f "$BINARY" ]; then
  echo "Error: Binary not found at $BINARY" >&2
  echo "Run without --no-build or compile first using 'cargo build --release'." >&2
  exit 1
fi

# 2. Validate desktop entry if tool is installed
if command -v desktop-file-validate >/dev/null 2>&1; then
  echo "==> Validating desktop file..."
  desktop-file-validate "$REPO_ROOT/packaging/shark.desktop"
fi

# 3. Assemble AppDir
APPDIR="$REPO_ROOT/target/Shark.AppDir"
echo "==> Assembling AppDir at $APPDIR..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/lib" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"

install -Dm755 "$BINARY" "$APPDIR/usr/bin/shark"
install -Dm755 "$REPO_ROOT/packaging/AppRun" "$APPDIR/AppRun"
install -Dm644 "$REPO_ROOT/packaging/shark.desktop" "$APPDIR/usr/share/applications/shark.desktop"
install -Dm644 "$REPO_ROOT/assets/icons/shark.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/shark.svg"

# Resize PNG icon if imagemagick / convert is available, else copy
if command -v magick >/dev/null 2>&1; then
  magick "$REPO_ROOT/assets/icons/shark.png" -resize 256x256 "$APPDIR/usr/share/icons/hicolor/256x256/apps/shark.png"
elif command -v convert >/dev/null 2>&1; then
  convert "$REPO_ROOT/assets/icons/shark.png" -resize 256x256 "$APPDIR/usr/share/icons/hicolor/256x256/apps/shark.png"
else
  cp "$REPO_ROOT/assets/icons/shark.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/shark.png"
fi

# AppDir standard symlinks
ln -sf usr/share/applications/shark.desktop "$APPDIR/shark.desktop"
ln -sf usr/share/icons/hicolor/256x256/apps/shark.png "$APPDIR/shark.png"
ln -sf shark.png "$APPDIR/.DirIcon"

# Bundle core XKB libraries for maximum cross-distro compatibility (Wayland / X11)
if command -v ldconfig >/dev/null 2>&1; then
  echo "==> Bundling compatible XKB runtime libraries..."
  for lib in libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1; do
    src=$(ldconfig -p 2>/dev/null | awk -v name="$lib" '$1 == name && !found++ { print $NF }' || true)
    if [ -n "$src" ] && [ -f "$src" ]; then
      install -Dm755 "$src" "$APPDIR/usr/lib/$lib"
    fi
  done
fi

# 4. Locate or download appimagetool
APPIMAGE_TOOL=""
if command -v appimagetool >/dev/null 2>&1; then
  APPIMAGE_TOOL="appimagetool"
elif [ -x "$REPO_ROOT/target/appimagetool" ]; then
  APPIMAGE_TOOL="$REPO_ROOT/target/appimagetool"
elif [ -x "/tmp/appimagetool" ]; then
  APPIMAGE_TOOL="/tmp/appimagetool"
else
  echo "==> appimagetool not found locally. Downloading continuous release..."
  mkdir -p "$REPO_ROOT/target"
  APPIMAGE_TOOL="$REPO_ROOT/target/appimagetool"
  curl --fail --silent --show-error --location --retry 3 \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage \
    --output "$APPIMAGE_TOOL"
  chmod +x "$APPIMAGE_TOOL"
fi

# 5. Build AppImage
mkdir -p "$OUTPUT_DIR"
OUTPUT_FILE="$OUTPUT_DIR/Shark-x86_64.AppImage"
echo "==> Packaging into $OUTPUT_FILE..."
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGE_TOOL" "$APPDIR" "$OUTPUT_FILE"

echo ""
echo "==> Done! AppImage created at:"
echo "    $OUTPUT_FILE"
echo ""
echo "To test run:"
echo "    $OUTPUT_FILE"
