#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

input=$(cat)

if command -v jq &> /dev/null; then
  session_id=$(echo "$input" | jq -r '.session_id // ""')
  source=$(echo "$input" | jq -r '.source // ""')
else
  session_id=$(echo "$input" | grep -oE '"session_id" *: *"[^"]*"' | grep -oE '"[^"]*"$' | tr -d '"') || session_id=""
  source=$(echo "$input" | grep -oE '"source" *: *"[^"]*"' | grep -oE '"[^"]*"$' | tr -d '"') || source=""
fi

session_id=$(echo "$session_id" | tr -cd 'a-zA-Z0-9_-')

if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -n "${session_id:-}" ]; then
  echo "export CLAUDE_SESSION_ID=\"${session_id}\"" >> "$CLAUDE_ENV_FILE"
fi

# Philosophy prompt injection (single tier — plugin CLAUDE.md is not loaded in consumer projects)
if [ -n "$source" ]; then
  cat "$SCRIPT_DIR/philosophy-prompt.md"
fi

exit 0
