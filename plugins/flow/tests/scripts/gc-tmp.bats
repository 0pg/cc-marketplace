#!/usr/bin/env bats

SCRIPT="$BATS_TEST_DIRNAME/../../scripts/gc-tmp.sh"

setup() {
  TMPROOT="$(mktemp -d)/flow"
  mkdir -p "$TMPROOT"
  export FLOW_TMP_ROOT="$TMPROOT"
  export FLOW_TMP_KEEP_DAYS=7
  export CLAUDE_SESSION_ID="current-session"

  mkdir -p "$TMPROOT/current-session"
  mkdir -p "$TMPROOT/old-session"
  mkdir -p "$TMPROOT/recent-session"
  touch -t $(date -u -v-10d +%Y%m%d%H%M 2>/dev/null || date -u -d '10 days ago' +%Y%m%d%H%M) "$TMPROOT/old-session" 2>/dev/null || true
  # macOS/BSD date uses -v; GNU uses -d. Above tries macOS first, falls back to GNU.
  # If both fail (unlikely), set a file older than 7 days manually:
  find "$TMPROOT/old-session" -exec touch -d "10 days ago" {} \; 2>/dev/null || true
}

teardown() {
  rm -rf "$(dirname "$TMPROOT")"
}

@test "removes sessions older than keep-days" {
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ ! -d "$TMPROOT/old-session" ]
}

@test "preserves current session regardless of age" {
  find "$TMPROOT/current-session" -exec touch -d "30 days ago" {} \; 2>/dev/null || \
    touch -t $(date -u -v-30d +%Y%m%d%H%M 2>/dev/null) "$TMPROOT/current-session"
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ -d "$TMPROOT/current-session" ]
}

@test "preserves sessions younger than keep-days" {
  run "$SCRIPT"
  [ "$status" -eq 0 ]
  [ -d "$TMPROOT/recent-session" ]
}

@test "no-op when FLOW_TMP_ROOT does not exist" {
  rm -rf "$TMPROOT"
  run "$SCRIPT"
  [ "$status" -eq 0 ]
}
