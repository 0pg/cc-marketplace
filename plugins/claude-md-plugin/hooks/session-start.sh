#!/bin/bash
set -euo pipefail

input=$(cat)

if command -v jq &> /dev/null; then
  session_id=$(echo "$input" | jq -r '.session_id // ""')
else
  session_id=$(echo "$input" | grep -oE '"session_id" *: *"[^"]*"' | grep -oE '"[^"]*"$' | tr -d '"') || session_id=""
fi

session_id=$(echo "$session_id" | tr -cd 'a-zA-Z0-9_-')

if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -n "${session_id:-}" ]; then
  echo "export CLAUDE_SESSION_ID=\"${session_id}\"" >> "$CLAUDE_ENV_FILE"
fi

exit 0
