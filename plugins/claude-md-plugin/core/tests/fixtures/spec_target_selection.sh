#!/usr/bin/env bash
# Harness for Step 2.1e target selection logic.
# Inputs (env):
#   TMP_DIR  - directory containing verdict-aggregate.jsonl; outputs also written here
#   NO_ASK   - "true" when --no-ask is set
# Outputs (files in TMP_DIR):
#   target-path.txt     - set when exactly one auto_executable candidate
#   halt-reason.txt     - set on halt (count != 1 with NO_ASK=true, or multi auto)
#   ask-question.txt    - set when interactive ask path chosen
set -euo pipefail

: "${TMP_DIR:?TMP_DIR required}"
: "${NO_ASK:=false}"

# Ensure trailing slash semantics match SKILL usage
case "$TMP_DIR" in
  */) ;;
  *) TMP_DIR="$TMP_DIR/" ;;
esac

emit_halt() {
  printf '%b\n' "$1" > "${TMP_DIR}halt-reason.txt"
}

ask_user_with_reasons() {
  printf '%s\n' "$1" > "${TMP_DIR}ask-question.txt"
}

auto_ok=$(jq -c 'select(.execution=="auto_executable" and .target != ".")' \
           "${TMP_DIR}verdict-aggregate.jsonl")
count=$(echo "$auto_ok" | awk 'NF' | wc -l | tr -d ' ')

case "$count" in
  1)
    target_path=$(echo "$auto_ok" | jq -r '.target')
    printf '%s\n' "$target_path" > "${TMP_DIR}target-path.txt"
    ;;
  0)
    reasons=$(jq -r 'select(.target != ".") | "- \(.target): [\(.execution)] \(.reason)"' \
               "${TMP_DIR}verdict-aggregate.jsonl")
    if [ "$NO_ASK" = "true" ]; then
      emit_halt "no auto-executable target; PM/PO verdicts:\n$reasons"
    else
      ask_user_with_reasons "$reasons"
    fi
    ;;
  *)
    conflicts=$(echo "$auto_ok" | jq -r '"- \(.target): \(.reason)"')
    if [ "$NO_ASK" = "true" ]; then
      emit_halt "multiple nodes claim ownership:\n$conflicts"
    else
      ask_user_with_reasons "$conflicts"
    fi
    ;;
esac
