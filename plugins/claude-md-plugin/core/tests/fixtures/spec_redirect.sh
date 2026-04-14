#!/usr/bin/env bash
# Harness for Step 2.1 redirect loop with visited-set cycle detection.
#
# Inputs (env):
#   TMP_DIR         - working directory
#   INITIAL_TARGET  - starting target_path
#   ROUNDS_DIR      - directory containing pre-built mock verdict-aggregate JSONL files
#                     named: round-{N}.jsonl (N=1..)  The harness copies each to
#                     ${TMP_DIR}verdict-aggregate.jsonl before selecting.
#   MAX_ROUNDS      - runaway bug-guard (default 10)
#
# Outputs (files in TMP_DIR):
#   target-path.txt     - final selected target (on success)
#   halt-reason.txt     - halt reason (on cycle / missing target / runaway)
#   visited-trace.txt   - arrow-joined visited list (for inspection)
#   rounds-consumed.txt - number of rounds iterated
set -euo pipefail

: "${TMP_DIR:?TMP_DIR required}"
: "${INITIAL_TARGET:?INITIAL_TARGET required}"
: "${ROUNDS_DIR:?ROUNDS_DIR required}"
: "${MAX_ROUNDS:=10}"

case "$TMP_DIR" in
  */) ;;
  *) TMP_DIR="$TMP_DIR/" ;;
esac

emit_halt() {
  printf '%s\n' "$1" > "${TMP_DIR}halt-reason.txt"
}

target_path="$INITIAL_TARGET"
visited=()
round=1

while :; do
  # Simulate Step 2.1 dispatch+aggregate: load this round's mock JSONL
  src="${ROUNDS_DIR}/round-${round}.jsonl"
  if [ ! -f "$src" ]; then
    emit_halt "harness: missing mock round file $src"
    break
  fi
  cp "$src" "${TMP_DIR}verdict-aggregate.jsonl"

  # Step 2.1e selection (simplified: pick unique auto_executable for the current target)
  selected=$(jq -r --arg tp "$target_path" \
             'select(.target==$tp and .execution=="auto_executable") | .target' \
             "${TMP_DIR}verdict-aggregate.jsonl" | head -n1)
  if [ -z "$selected" ]; then
    emit_halt "no auto_executable verdict for $target_path at round $round"
    break
  fi

  visited+=("$target_path")

  redirect_to=$(jq -r --arg tp "$target_path" \
                'select(.target==$tp) | .redirect_to // empty' \
                "${TMP_DIR}verdict-aggregate.jsonl" | head -n1)

  if [ -n "$redirect_to" ]; then
    # Cycle check (safety net — a loop is a bug, not a convergence signal)
    for v in "${visited[@]}"; do
      if [ "$v" = "$redirect_to" ]; then
        trace=""
        for x in "${visited[@]}"; do
          if [ -z "$trace" ]; then trace="$x"; else trace="$trace → $x"; fi
        done
        trace="$trace → $redirect_to"
        printf '%s\n' "$trace" > "${TMP_DIR}visited-trace.txt"
        emit_halt "redirect cycle: $trace"
        printf '%s\n' "$round" > "${TMP_DIR}rounds-consumed.txt"
        exit 0
      fi
    done

    # Runaway bug-guard
    if [ "${#visited[@]}" -gt "$MAX_ROUNDS" ]; then
      emit_halt "redirect depth exceeded safety limit (bug guard)"
      printf '%s\n' "$round" > "${TMP_DIR}rounds-consumed.txt"
      exit 0
    fi

    target_path="$redirect_to"
    round=$((round + 1))
    continue
  fi

  # No redirect — authority converged
  printf '%s\n' "$target_path" > "${TMP_DIR}target-path.txt"
  trace=""
  for x in "${visited[@]}"; do
    if [ -z "$trace" ]; then trace="$x"; else trace="$trace → $x"; fi
  done
  printf '%s\n' "$trace" > "${TMP_DIR}visited-trace.txt"
  printf '%s\n' "$round" > "${TMP_DIR}rounds-consumed.txt"
  break
done
