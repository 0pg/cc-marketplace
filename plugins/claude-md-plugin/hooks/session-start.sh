#!/bin/bash
set -euo pipefail

# claude-md-plugin SessionStart hook — v19
#
# Fires on: startup, resume, clear, compact (matcher "*" in hooks.json).
# Emits the node-agent tree philosophy reminder on stdout so it is
# injected into the session's context (visible in transcript).

# Drain stdin — Claude Code pipes a JSON payload we don't need.
cat > /dev/null

# Resolve the reminder path. ${CLAUDE_PLUGIN_ROOT} is provided by Claude
# Code when running plugin hooks; fall back to script directory so the
# hook also works outside the plugin harness (for local testing).
REMINDER_FILE="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}/hooks/philosophy-reminder.md"

if [ -f "$REMINDER_FILE" ]; then
  cat "$REMINDER_FILE"
fi

exit 0
