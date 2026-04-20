#!/bin/bash
# flow gc-tmp.sh
# Prune /tmp/flow/{session-id}/ directories older than FLOW_TMP_KEEP_DAYS (default 7),
# preserving the current session ($CLAUDE_SESSION_ID).
#
# Variables (all optional, sensible defaults):
#   FLOW_TMP_ROOT      — default "/tmp/flow"
#   FLOW_TMP_KEEP_DAYS — default 7
#   CLAUDE_SESSION_ID  — preserved if set
set -euo pipefail

tmp_root="${FLOW_TMP_ROOT:-/tmp/flow}"
keep_days="${FLOW_TMP_KEEP_DAYS:-7}"
current="${CLAUDE_SESSION_ID:-}"

[ -d "$tmp_root" ] || exit 0

# `find -mtime +N` matches entries strictly older than N days.
while IFS= read -r -d '' d; do
  name=$(basename "$d")
  if [ -n "$current" ] && [ "$name" = "$current" ]; then
    continue
  fi
  rm -rf "$d"
done < <(find "$tmp_root" -mindepth 1 -maxdepth 1 -type d -mtime +"$keep_days" -print0 2>/dev/null)
