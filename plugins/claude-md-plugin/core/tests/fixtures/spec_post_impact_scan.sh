#!/usr/bin/env bash
# Harness for /spec Step 4.5: surface affected consumers on schema change.
#
# Replicates the Step 4.5 block from skills/spec/SKILL.md against fixture
# DEVELOPERS.md files, invoking the actual built core binary for both
# detect-schema-change and impact-scan.
#
# Inputs (env):
#   TMP_DIR        - working directory (trailing slash enforced)
#   CORE_BIN       - path to built claude-md-core binary
#   TARGET_ROOT    - project root containing target + consumer dirs
#   TARGET_PATH    - relative target path (e.g. "producer")
#   BEFORE_FILE    - DEVELOPERS.md "before" snapshot
#   AFTER_FILE     - DEVELOPERS.md "after" snapshot (equals TARGET_ROOT/TARGET_PATH/DEVELOPERS.md)
#
# Outputs:
#   ${TMP_DIR}result-block.md  - synthesized result block (may or may not contain Affected Consumers)
set -euo pipefail

: "${TMP_DIR:?TMP_DIR required}"
: "${CORE_BIN:?CORE_BIN required}"
: "${TARGET_ROOT:?TARGET_ROOT required}"
: "${TARGET_PATH:?TARGET_PATH required}"
: "${BEFORE_FILE:?BEFORE_FILE required}"
: "${AFTER_FILE:?AFTER_FILE required}"

case "$TMP_DIR" in
  */) ;;
  *) TMP_DIR="$TMP_DIR/" ;;
esac

# Seed a minimal result block so the test has something to inspect.
: > "${TMP_DIR}result-block.md"
{
  echo "---spec-result---"
  echo "modules:"
  echo "  - ${TARGET_PATH}: ok (updated)"
} >> "${TMP_DIR}result-block.md"

# Step 4.5 logic
changed_json=$("$CORE_BIN" detect-schema-change --before "$BEFORE_FILE" --after "$AFTER_FILE")
changed=$(printf '%s' "$changed_json" | sed -nE 's/.*"changed":[[:space:]]*(true|false).*/\1/p')

if [ "$changed" = "true" ]; then
  "$CORE_BIN" impact-scan --target "$TARGET_PATH" --root "$TARGET_ROOT" --format list \
    > "${TMP_DIR}affected-consumers.txt"
  if [ -s "${TMP_DIR}affected-consumers.txt" ]; then
    {
      echo ""
      echo "## Affected Consumers"
      while IFS= read -r c; do
        [ -n "$c" ] && echo "- $c"
      done < "${TMP_DIR}affected-consumers.txt"
      echo ""
      echo "> Recommend \`/sync\` each consumer, or \`/autodev --auto-sync\` to delegate."
    } >> "${TMP_DIR}result-block.md"
  fi
fi
