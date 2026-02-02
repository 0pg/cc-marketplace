#!/bin/bash
# build-cli.sh - Rust CLI 빌드 헬퍼
#
# Usage: ./build-cli.sh [--release|--debug]

set -e

BUILD_MODE="${1:---release}"

PLUGIN_ROOT="$(dirname "$(dirname "$(realpath "$0")")")"
CORE_DIR="$PLUGIN_ROOT/core"

if [ ! -d "$CORE_DIR" ]; then
  echo "Error: Core directory not found: $CORE_DIR"
  exit 1
fi

cd "$CORE_DIR"

case "$BUILD_MODE" in
  --release)
    echo "Building claude-md-core (release)..."
    cargo build --release
    echo ""
    echo "Binary: $CORE_DIR/target/release/claude-md-core"
    ;;
  --debug)
    echo "Building claude-md-core (debug)..."
    cargo build
    echo ""
    echo "Binary: $CORE_DIR/target/debug/claude-md-core"
    ;;
  *)
    echo "Usage: $0 [--release|--debug]"
    exit 1
    ;;
esac

echo ""
echo "Build complete."
