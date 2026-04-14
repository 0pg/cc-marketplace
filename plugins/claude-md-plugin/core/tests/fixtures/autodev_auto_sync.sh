#!/usr/bin/env bash
# Harness for /autodev --auto-sync Step 4.7: consumer propagation.
#
# Replicates the Step 4.7 behavior against mock po-consultant result files
# and a mock `sync` shim that only logs invocations. Executes each consumer's
# verdict verbatim — `auto_executable` invokes the sync shim, anything else
# halts the chain with the reason preserved.
#
# Inputs (env):
#   TMP_DIR           - working directory (trailing slash enforced)
#   CONSUMERS_FILE    - path to affected-consumers.txt (one path per line)
#   FIXTURE_DIR       - directory containing {consumer}.result.md mock files
#
# Outputs:
#   ${TMP_DIR}sync-invocations.log - one line per synced consumer
#   ${TMP_DIR}result-block.md      - synthesized result block with ## Sync Results
set -euo pipefail

: "${TMP_DIR:?TMP_DIR required}"
: "${CONSUMERS_FILE:?CONSUMERS_FILE required}"
: "${FIXTURE_DIR:?FIXTURE_DIR required}"

case "$TMP_DIR" in
  */) ;;
  *) TMP_DIR="$TMP_DIR/" ;;
esac

SYNC_LOG="${TMP_DIR}sync-invocations.log"
RESULT_BLOCK="${TMP_DIR}result-block.md"
: > "$SYNC_LOG"
: > "$RESULT_BLOCK"

# Mock sync shim: log the consumer path instead of actually running /sync.
run_sync() {
  echo "$1" >> "$SYNC_LOG"
}

extract_field() {
  # $1 = file, $2 = section header (e.g. "Execution")
  awk -v hdr="## $2" '
    $0 == hdr { capture=1; next }
    capture && /^## / { exit }
    capture {
      # trim leading/trailing whitespace
      gsub(/^[[:space:]]+|[[:space:]]+$/, "")
      if ($0 != "") { print; exit }
    }
  ' "$1"
}

echo "" >> "$RESULT_BLOCK"
echo "## Sync Results" >> "$RESULT_BLOCK"

status="synced_all"
halt_reason=""
halt_consumer=""

while IFS= read -r consumer; do
  [ -z "$consumer" ] && continue

  result_file="${FIXTURE_DIR}/${consumer}.result.md"
  if [ ! -f "$result_file" ]; then
    echo "missing consult-result for $consumer: $result_file" >&2
    exit 2
  fi

  execution=$(extract_field "$result_file" "Execution")
  reason=$(extract_field "$result_file" "Reason")

  if [ "$execution" = "auto_executable" ]; then
    run_sync "$consumer"
    echo "- ${consumer}: synced" >> "$RESULT_BLOCK"
  else
    status="halted"
    halt_consumer="$consumer"
    halt_reason="$reason"
    echo "- ${consumer}: halted: ${reason}" >> "$RESULT_BLOCK"
    break
  fi
done < "$CONSUMERS_FILE"

if [ "$status" = "halted" ]; then
  {
    echo ""
    echo "status: halted"
    echo "halted_at: ${halt_consumer}"
    echo "reason: ${halt_reason}"
    echo ""
    echo "> Rollback hint: \`git revert HEAD\`"
  } >> "$RESULT_BLOCK"
else
  {
    echo ""
    echo "status: synced_all"
  } >> "$RESULT_BLOCK"
fi
