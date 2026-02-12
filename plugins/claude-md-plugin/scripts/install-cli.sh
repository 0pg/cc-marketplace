#!/bin/bash
set -euo pipefail

# claude-md-core CLI ensure script
# Checks cargo installation and builds CLI binary if needed
#
# Usage:
#   install-cli.sh                    # Ensure CLI is ready
#   install-cli.sh --check            # Check current status only
#   install-cli.sh --cli-path-only    # Print CLI_PATH and exit

# Resolve CORE_DIR
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_DIR="$PLUGIN_ROOT/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"

# Color output (if terminal supports it)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [[ ! -t 1 ]]; then
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

# Parse options
CHECK_ONLY=false
CLI_PATH_ONLY=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --check)
      CHECK_ONLY=true
      shift
      ;;
    --cli-path-only)
      CLI_PATH_ONLY=true
      shift
      ;;
    -h|--help)
      echo "Usage: install-cli.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --check          Check current status without making changes"
      echo "  --cli-path-only  Print CLI binary path and exit"
      echo "  -h, --help       Show this help message"
      exit 0
      ;;
    *)
      echo "Error: Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# --cli-path-only: print expected path (does not verify existence)
if $CLI_PATH_ONLY; then
  echo "$CLI_PATH"
  exit 0
fi

# Check cargo availability
check_cargo() {
  if command -v cargo &> /dev/null; then
    return 0
  fi
  # Try sourcing cargo env in case it was installed but not in PATH
  if [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
    if command -v cargo &> /dev/null; then
      return 0
    fi
  fi
  return 1
}

# Check CLI binary
check_cli() {
  if [[ -f "$CLI_PATH" ]]; then
    return 0
  fi
  return 1
}

# --check mode
if $CHECK_ONLY; then
  echo "=== claude-md-core CLI Status ==="
  echo ""
  if check_cargo; then
    echo -e "${GREEN}[OK] cargo is installed${NC}"
    echo "     $(cargo --version 2>/dev/null)"
  else
    echo -e "${YELLOW}[--] cargo is not installed${NC}"
  fi
  if check_cli; then
    echo -e "${GREEN}[OK] claude-md-core binary exists${NC}"
    echo "     $CLI_PATH"
  else
    echo -e "${YELLOW}[--] claude-md-core binary not found${NC}"
  fi
  exit 0
fi

# Ensure mode: check and build
if check_cli; then
  echo -e "${GREEN}claude-md-core is ready${NC}" >&2
  echo "$CLI_PATH"
  exit 0
fi

# CLI binary not found - need to build
echo -e "${YELLOW}claude-md-core binary not found. Building...${NC}" >&2

# Check core directory exists
if [ ! -d "$CORE_DIR" ]; then
  echo -e "${RED}Error: core directory not found: $CORE_DIR${NC}" >&2
  exit 1
fi

# Check cargo
if ! check_cargo; then
  echo -e "${RED}Error: Rust/Cargo is not installed${NC}" >&2
  echo "" >&2
  echo "Install Rust:" >&2
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
  echo "" >&2
  echo "For more information:" >&2
  echo "  https://www.rust-lang.org/tools/install" >&2
  exit 1
fi

# Build
echo "Building claude-md-core (release)..." >&2
if (cd "$CORE_DIR" && cargo build --release >&2); then
  echo -e "${GREEN}Build successful${NC}" >&2
  echo "$CLI_PATH"
else
  echo -e "${RED}Build failed${NC}" >&2
  echo "Check build logs in: $CORE_DIR" >&2
  exit 1
fi
