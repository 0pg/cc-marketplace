#!/bin/bash
# test-skill.sh - 스킬 테스트 헬퍼
#
# Usage: ./test-skill.sh <skill_name> [options]
# Example: ./test-skill.sh code-analyze --language typescript

set -e

SKILL_NAME="${1:-}"
shift || true

if [ -z "$SKILL_NAME" ]; then
  echo "Usage: $0 <skill_name> [options]"
  echo ""
  echo "Available skills:"
  echo "  - extract"
  echo "  - tree-parse"
  echo "  - boundary-resolve"
  echo "  - code-analyze"
  echo "  - draft-generate"
  echo "  - schema-validate"
  echo ""
  echo "Options for code-analyze:"
  echo "  --language <lang>  Test specific language (typescript, python, go, rust, java, kotlin)"
  exit 1
fi

PLUGIN_ROOT="$(dirname "$(dirname "$(realpath "$0")")")"
SKILL_DIR="$PLUGIN_ROOT/skills/$SKILL_NAME"

if [ ! -d "$SKILL_DIR" ]; then
  echo "Error: Skill not found: $SKILL_NAME"
  exit 1
fi

echo "Testing skill: $SKILL_NAME"
echo "Skill directory: $SKILL_DIR"
echo ""

# Check for Gherkin tests
FEATURE_FILE="$SKILL_DIR/tests/${SKILL_NAME//-/_}.feature"
if [ -f "$FEATURE_FILE" ]; then
  echo "Found feature file: $FEATURE_FILE"
  echo "Run Gherkin tests with: cargo test --test cucumber"
else
  echo "No feature file found at: $FEATURE_FILE"
fi

# Check for examples
EXAMPLES_DIR="$SKILL_DIR/examples"
if [ -d "$EXAMPLES_DIR" ]; then
  echo ""
  echo "Available examples:"
  ls -la "$EXAMPLES_DIR"
fi

# Language-specific test for code-analyze
if [ "$SKILL_NAME" = "code-analyze" ]; then
  LANGUAGE=""
  while [[ $# -gt 0 ]]; do
    case $1 in
      --language)
        LANGUAGE="$2"
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done

  if [ -n "$LANGUAGE" ]; then
    FIXTURE_DIR="$PLUGIN_ROOT/fixtures/$LANGUAGE"
    if [ -d "$FIXTURE_DIR" ]; then
      echo ""
      echo "Testing with $LANGUAGE fixtures from: $FIXTURE_DIR"
      ls -la "$FIXTURE_DIR"
    else
      echo "Warning: No fixtures found for $LANGUAGE"
    fi
  fi
fi
