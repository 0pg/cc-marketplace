#!/bin/bash
set -euo pipefail

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT}"
CORE_DIR="${PLUGIN_ROOT}/core"
BINARY="${CORE_DIR}/target/release/claude-md-core"
CARGO_TOML="${CORE_DIR}/Cargo.toml"
CARGO_LOCK="${CORE_DIR}/Cargo.lock"

# core 디렉토리 없으면 skip
if [ ! -d "$CORE_DIR" ]; then
  exit 0
fi

# cargo 없으면 graceful skip
if ! command -v cargo &> /dev/null; then
  echo "Warning: cargo not found, claude-md-core features unavailable" >&2
  exit 0
fi

# 빌드 필요 여부 확인: 바이너리 없거나 Cargo 파일이 더 최신이면 빌드
needs_build=false
if [ ! -f "$BINARY" ]; then
  needs_build=true
elif [ "$CARGO_TOML" -nt "$BINARY" ]; then
  needs_build=true
elif [ -f "$CARGO_LOCK" ] && [ "$CARGO_LOCK" -nt "$BINARY" ]; then
  needs_build=true
elif [ -d "$CORE_DIR/src" ] && [ -n "$(find "$CORE_DIR/src" -name '*.rs' -newer "$BINARY" 2>/dev/null)" ]; then
  needs_build=true
fi

if [ "$needs_build" = true ]; then
  echo "Building claude-md-core..." >&2
  if ! (cd "$CORE_DIR" && cargo build --release >&2); then
    echo "Warning: Failed to build claude-md-core" >&2
  fi
fi

exit 0  # 세션 시작 차단하지 않음
