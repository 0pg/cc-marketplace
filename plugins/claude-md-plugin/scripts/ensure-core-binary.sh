#!/bin/bash
set -euo pipefail

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT}"
BINARY="${PLUGIN_ROOT}/core/target/release/claude-md-core"
CARGO_TOML="${PLUGIN_ROOT}/core/Cargo.toml"

# cargo 없으면 graceful skip
if ! command -v cargo &> /dev/null; then
  echo "Warning: cargo not found, claude-md-core features unavailable" >&2
  exit 0
fi

# 바이너리 없거나 Cargo.toml이 더 최신이면 빌드
if [ ! -f "$BINARY" ] || [ "$CARGO_TOML" -nt "$BINARY" ]; then
  echo "Building claude-md-core..." >&2
  if ! (cd "${PLUGIN_ROOT}/core" && cargo build --release 2>&1 | grep -E "(Compiling|Finished|error)" >&2); then
    echo "Warning: Failed to build claude-md-core" >&2
  fi
fi

exit 0  # 세션 시작 차단하지 않음
