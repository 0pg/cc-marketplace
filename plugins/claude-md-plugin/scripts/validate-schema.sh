#!/bin/bash
# validate-schema.sh - CLAUDE.md 스키마 독립 검증
#
# Usage: ./validate-schema.sh <file_path>
# Example: ./validate-schema.sh src/auth/CLAUDE.md

set -e

FILE_PATH="${1:-}"

if [ -z "$FILE_PATH" ]; then
  echo "Usage: $0 <file_path>"
  echo "Example: $0 src/auth/CLAUDE.md"
  exit 1
fi

if [ ! -f "$FILE_PATH" ]; then
  echo "Error: File not found: $FILE_PATH"
  exit 1
fi

PLUGIN_ROOT="$(dirname "$(dirname "$(realpath "$0")")")"
CLI_PATH="$PLUGIN_ROOT/core/target/release/claude-md-core"

if [ ! -f "$CLI_PATH" ]; then
  echo "Building claude-md-core..."
  cd "$PLUGIN_ROOT/core" && cargo build --release
fi

echo "Validating: $FILE_PATH"
"$CLI_PATH" validate-schema --file "$FILE_PATH" --output /dev/stdout

echo ""
echo "Validation complete."
