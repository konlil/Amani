#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$PROJECT_ROOT/app"
ENGINE_BIN_DIR="$PROJECT_ROOT/engine/godot/bin"
TAURI_BUNDLE_DIR="$APP_DIR/src-tauri/target/release/bundle"

BUILD_ENGINE=1
BUILD_APP=1
PLATFORM="macos"
TARGET="template_release"

usage() {
  cat <<'EOF'
Usage: ./build/build_all_macos.sh [--engine-only] [--app-only] [--platform <name>] [--target <name>]

Build outputs for the macOS workspace:
  - Engine binary: engine/godot/bin/
  - Desktop app bundle: app/src-tauri/target/release/bundle/
EOF
}

find_engine_binary() {
  find "$ENGINE_BIN_DIR" -maxdepth 1 -type f -name "godot.${PLATFORM}.${TARGET}*" | sort | head -n 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --engine-only)
      BUILD_ENGINE=1
      BUILD_APP=0
      shift
      ;;
    --app-only)
      BUILD_ENGINE=0
      BUILD_APP=1
      shift
      ;;
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --target)
      TARGET="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[ERROR] Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "[ERROR] This build script currently targets macOS only."
  exit 1
fi

echo "=== LLM3dEngine Build (macOS) ==="
echo "Project root: $PROJECT_ROOT"
echo "Platform:     $PLATFORM"
echo "Target:       $TARGET"
echo ""

if [[ $BUILD_ENGINE -eq 1 ]]; then
  echo "[1/2] Building Godot engine..."
  "$SCRIPT_DIR/macos.sh" "$PLATFORM" "$TARGET"
else
  echo "[1/2] Skipping engine build."
fi

if [[ $BUILD_APP -eq 1 ]]; then
  echo "[2/2] Building Tauri desktop app..."
  (cd "$APP_DIR" && npm run tauri build)
else
  echo "[2/2] Skipping Tauri app build."
fi

ENGINE_BINARY="$(find_engine_binary || true)"

echo ""
echo "=== Build Complete ==="
if [[ -n "$ENGINE_BINARY" ]]; then
  echo "Engine binary: $ENGINE_BINARY"
else
  echo "Engine binary: not found under $ENGINE_BIN_DIR"
fi

if [[ -d "$TAURI_BUNDLE_DIR" ]]; then
  echo "Desktop bundle dir: $TAURI_BUNDLE_DIR"
fi
