#!/bin/bash
# flow plugin session-start hook.
# Scans .claude/workflows/flow/*/state.json for tasks with status in {running, halted}
# and surfaces a one-line notice. Non-blocking.
set -euo pipefail

# Discard stdin (hook input JSON — we don't need it).
cat > /dev/null || true

# Only runs inside a repo with a flow workflow directory. Bail silently otherwise.
workflow_root=".claude/workflows/flow"
if [ ! -d "$workflow_root" ]; then
  exit 0
fi

in_progress=""
halted=""
count_running=0
count_halted=0

# Use find for portability. State files are small; this is <1s on realistic repos.
while IFS= read -r -d '' state_file; do
  task_id=$(basename "$(dirname "$state_file")")
  status=""
  if command -v jq >/dev/null 2>&1; then
    status=$(jq -r '.status // ""' "$state_file" 2>/dev/null || echo "")
  else
    # Fallback: grep for the top-level status field.
    status=$(grep -oE '"status"[[:space:]]*:[[:space:]]*"[^"]*"' "$state_file" | head -1 | sed -E 's/.*"([^"]*)"$/\1/') || status=""
  fi

  case "$status" in
    running)
      in_progress="${in_progress}${in_progress:+, }${task_id}"
      count_running=$((count_running + 1))
      ;;
    halted)
      halted="${halted}${halted:+, }${task_id}"
      count_halted=$((count_halted + 1))
      ;;
  esac
done < <(find "$workflow_root" -mindepth 2 -maxdepth 2 -name state.json -type f -print0 2>/dev/null)

if [ "$count_running" -gt 0 ] || [ "$count_halted" -gt 0 ]; then
  echo "flow: $count_running running, $count_halted halted. /flow-status for details."
fi

exit 0
