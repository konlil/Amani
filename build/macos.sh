#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GODOT_DIR="$PROJECT_ROOT/engine/godot"
CUSTOM_MODULES="$PROJECT_ROOT/modules"

# Platform detection
PLATFORM="${1:-macos}"
TARGET="${2:-template_release}"
JOBS=$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)

# Vulkan SDK path (macOS)
VULKAN_SDK_PATH="/opt/homebrew/opt/molten-vk"

echo "=== LLM3dEngine Build ==="
echo "Platform:       $PLATFORM"
echo "Target:         $TARGET"
echo "Jobs:           $JOBS"
echo "Godot dir:      $GODOT_DIR"
echo "Custom modules: $CUSTOM_MODULES"
echo ""

cd "$GODOT_DIR"

scons platform="$PLATFORM" target="$TARGET" \
  vulkan_sdk_path="$VULKAN_SDK_PATH" \
  tools=no \
  -j"$JOBS" \
  \
  module_visual_script_enabled=no \
  module_mono_enabled=no \
  module_mobile_vr_enabled=no \
  module_openxr_enabled=no \
  module_webxr_enabled=no \
  module_lightmapper_rd_enabled=no \
  module_raycast_enabled=no \
  module_theora_enabled=no \
  module_webrtc_enabled=no \
  module_cvtt_enabled=no \
  module_betsy_enabled=no \
  module_gridmap_enabled=no \
  module_csg_enabled=no \
  \
  custom_modules="$CUSTOM_MODULES"

echo ""
echo "=== Build Complete ==="
echo "Output:"
ls -lh "$GODOT_DIR/bin/"
