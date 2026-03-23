#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$PROJECT_ROOT/app"
TESTS_DIR="$PROJECT_ROOT/tests/e2e_test"

WITH_TESTS=0

usage() {
  cat <<'EOF'
Usage: ./build/bootstrap_macos.sh [--with-tests]

Initialize local dependencies for the macOS development environment:
  1. Verify required system tools
  2. Initialize git submodules
  3. Install npm dependencies for the desktop app
  4. Optionally install npm dependencies for tests/e2e_test
EOF
}

require_command() {
  local cmd="$1"
  local hint="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[ERROR] Missing command: $cmd"
    echo "        $hint"
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --with-tests)
      WITH_TESTS=1
      shift
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
  echo "[ERROR] This bootstrap script currently targets macOS only."
  exit 1
fi

echo "=== LLM3dEngine Bootstrap (macOS) ==="
echo "Project root: $PROJECT_ROOT"
echo ""

echo "[1/4] Checking system prerequisites..."
if ! xcode-select -p >/dev/null 2>&1; then
  echo "[ERROR] Xcode Command Line Tools are not installed."
  echo "        Run: xcode-select --install"
  exit 1
fi

require_command git "Install Xcode Command Line Tools or Git first."
require_command node "Install Node.js LTS first."
require_command npm "Install Node.js LTS first."
require_command python3 "Install Python 3 first."
require_command cargo "Install Rust with rustup: https://rustup.rs/"
require_command scons "Install SCons, for example: brew install scons"

if [[ ! -d /opt/homebrew/opt/molten-vk && ! -d /usr/local/opt/molten-vk ]]; then
  echo "[ERROR] MoltenVK was not found."
  echo "        Install it with Homebrew: brew install molten-vk"
  exit 1
fi

echo "[2/4] Initializing git submodules..."
git -C "$PROJECT_ROOT" submodule update --init --recursive

echo "[3/4] Installing app dependencies with npm ci..."
(cd "$APP_DIR" && npm ci)

if [[ $WITH_TESTS -eq 1 ]]; then
  echo "[4/4] Installing test dependencies with npm ci..."
  (cd "$TESTS_DIR" && npm ci)
else
  echo "[4/4] Skipping tests/e2e_test dependencies. Pass --with-tests to install them."
fi

echo ""
echo "=== Bootstrap Complete ==="
echo "Desktop app dependencies: $APP_DIR/node_modules"
if [[ $WITH_TESTS -eq 1 ]]; then
  echo "Test dependencies:        $TESTS_DIR/node_modules"
fi
