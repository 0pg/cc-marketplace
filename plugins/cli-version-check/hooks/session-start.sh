#!/bin/bash
# Session Start Hook - cli-version-check plugin
# Detects stale CLI binaries by comparing binary mtime with source file mtimes.
# Supports: Cargo (Rust), Go, npm/pnpm (Node.js), Make, Poetry/pip (Python)

set -euo pipefail

# Read JSON input from stdin
input=$(cat)
source_type=$(echo "$input" | jq -r '.source // "startup"' 2>/dev/null || echo "startup")

PROJECT_ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"

# ============================================================================
# Build System Detection
# ============================================================================

# Each detector outputs: binary_path|source_dir|build_cmd|label
# Multiple entries separated by newlines

detect_cargo() {
    local cargo_toml="$PROJECT_ROOT/Cargo.toml"
    [[ -f "$cargo_toml" ]] || return 0

    # Find all [[bin]] targets or infer from package name
    local bin_names=()

    if command -v cargo &>/dev/null; then
        # Use cargo metadata if available
        while IFS= read -r name; do
            [[ -n "$name" ]] && bin_names+=("$name")
        done < <(cargo metadata --no-deps --format-version=1 --manifest-path "$cargo_toml" 2>/dev/null \
            | jq -r '.packages[].targets[] | select(.kind[] == "bin") | .name' 2>/dev/null || true)
    fi

    # Fallback: parse package name from Cargo.toml
    if [[ ${#bin_names[@]} -eq 0 ]]; then
        local pkg_name
        pkg_name=$(grep -m1 '^name\s*=' "$cargo_toml" | sed 's/.*=\s*"\(.*\)".*/\1/' || true)
        [[ -n "$pkg_name" ]] && bin_names+=("$pkg_name")
    fi

    for name in "${bin_names[@]}"; do
        # Check common binary locations
        for bin_path in \
            "$PROJECT_ROOT/target/release/$name" \
            "$PROJECT_ROOT/target/debug/$name"; do
            if [[ -f "$bin_path" ]]; then
                echo "$bin_path|$PROJECT_ROOT/src|cargo build|Cargo: $name"
            fi
        done
    done
}

detect_go() {
    [[ -f "$PROJECT_ROOT/go.mod" ]] || return 0

    local module_name
    module_name=$(head -1 "$PROJECT_ROOT/go.mod" | awk '{print $2}' || true)
    local bin_name
    bin_name=$(basename "$module_name" 2>/dev/null || echo "")

    [[ -z "$bin_name" ]] && return 0

    # Check GOPATH/bin, ./bin, project root
    local gopath_bin="${GOPATH:-$HOME/go}/bin/$bin_name"
    for bin_path in \
        "$gopath_bin" \
        "$PROJECT_ROOT/bin/$bin_name" \
        "$PROJECT_ROOT/$bin_name"; do
        if [[ -f "$bin_path" ]]; then
            echo "$bin_path|$PROJECT_ROOT|go build|Go: $bin_name"
            return 0
        fi
    done

    # Check dist/ or build/ directories
    for dir in dist build out; do
        if [[ -d "$PROJECT_ROOT/$dir" ]]; then
            local found
            found=$(find "$PROJECT_ROOT/$dir" -maxdepth 2 -name "$bin_name" -type f -executable 2>/dev/null | head -1 || true)
            if [[ -n "$found" ]]; then
                echo "$found|$PROJECT_ROOT|go build|Go: $bin_name"
                return 0
            fi
        fi
    done
}

detect_node() {
    local pkg_json="$PROJECT_ROOT/package.json"
    [[ -f "$pkg_json" ]] || return 0

    # Check if package has a bin field
    local has_bin
    has_bin=$(jq -r 'has("bin")' "$pkg_json" 2>/dev/null || echo "false")
    [[ "$has_bin" == "true" ]] || return 0

    # Get bin entries
    local bin_type
    bin_type=$(jq -r '.bin | type' "$pkg_json" 2>/dev/null || echo "null")

    if [[ "$bin_type" == "string" ]]; then
        local bin_path
        bin_path=$(jq -r '.bin' "$pkg_json" 2>/dev/null || true)
        local pkg_name
        pkg_name=$(jq -r '.name' "$pkg_json" 2>/dev/null || true)
        if [[ -n "$bin_path" ]]; then
            # Check if there's a build step (dist/ or build output)
            local src_dir="$PROJECT_ROOT/src"
            [[ -d "$src_dir" ]] || src_dir="$PROJECT_ROOT"
            local build_cmd
            build_cmd=$(jq -r '.scripts.build // "npm run build"' "$pkg_json" 2>/dev/null || echo "npm run build")
            echo "$PROJECT_ROOT/$bin_path|$src_dir|$build_cmd|Node: $pkg_name"
        fi
    elif [[ "$bin_type" == "object" ]]; then
        while IFS= read -r entry; do
            local name path_val
            name=$(echo "$entry" | jq -r '.key' 2>/dev/null || true)
            path_val=$(echo "$entry" | jq -r '.value' 2>/dev/null || true)
            if [[ -n "$name" && -n "$path_val" ]]; then
                local src_dir="$PROJECT_ROOT/src"
                [[ -d "$src_dir" ]] || src_dir="$PROJECT_ROOT"
                local build_cmd
                build_cmd=$(jq -r '.scripts.build // "npm run build"' "$pkg_json" 2>/dev/null || echo "npm run build")
                echo "$PROJECT_ROOT/$path_val|$src_dir|$build_cmd|Node: $name"
            fi
        done < <(jq -c '.bin | to_entries[] | {key: .key, value: .value}' "$pkg_json" 2>/dev/null || true)
    fi
}

detect_make() {
    [[ -f "$PROJECT_ROOT/Makefile" ]] || return 0

    # Look for common binary output patterns in Makefile
    # e.g., BIN=./myapp, OUTPUT=build/myapp
    local bin_path
    bin_path=$(grep -E '^\s*(BIN|BINARY|OUTPUT|TARGET)\s*[:?]?=' "$PROJECT_ROOT/Makefile" \
        | head -1 | sed 's/.*[:?]\?=\s*//' | sed 's/#.*//' | xargs 2>/dev/null || true)

    if [[ -n "$bin_path" ]]; then
        # Resolve relative paths
        if [[ "$bin_path" != /* ]]; then
            bin_path="$PROJECT_ROOT/$bin_path"
        fi
        if [[ -f "$bin_path" ]]; then
            echo "$bin_path|$PROJECT_ROOT/src|make|Make: $(basename "$bin_path")"
        fi
    fi
}

detect_python() {
    # Check for pyproject.toml with [project.scripts]
    local pyproject="$PROJECT_ROOT/pyproject.toml"
    if [[ -f "$pyproject" ]]; then
        # Check if there's a scripts section (CLI entry points)
        if grep -q '\[project\.scripts\]' "$pyproject" 2>/dev/null; then
            # Find installed CLI in venv
            for venv_dir in .venv venv env; do
                local venv_bin="$PROJECT_ROOT/$venv_dir/bin"
                if [[ -d "$venv_bin" ]]; then
                    # Get script names from pyproject.toml
                    local script_name
                    script_name=$(sed -n '/\[project\.scripts\]/,/^\[/p' "$pyproject" \
                        | grep '=' | head -1 | cut -d= -f1 | xargs 2>/dev/null || true)
                    if [[ -n "$script_name" && -f "$venv_bin/$script_name" ]]; then
                        local src_dir="$PROJECT_ROOT/src"
                        [[ -d "$src_dir" ]] || src_dir="$PROJECT_ROOT"
                        echo "$venv_bin/$script_name|$src_dir|pip install -e .|Python: $script_name"
                    fi
                fi
            done
        fi
    fi

    # Check for setup.py with console_scripts
    if [[ -f "$PROJECT_ROOT/setup.py" ]]; then
        if grep -q 'console_scripts' "$PROJECT_ROOT/setup.py" 2>/dev/null; then
            for venv_dir in .venv venv env; do
                local venv_bin="$PROJECT_ROOT/$venv_dir/bin"
                if [[ -d "$venv_bin" ]]; then
                    local script_name
                    script_name=$(grep -oP "(?<=['\"])[^'\"]+(?=\s*=)" "$PROJECT_ROOT/setup.py" | head -1 || true)
                    if [[ -n "$script_name" && -f "$venv_bin/$script_name" ]]; then
                        local src_dir="$PROJECT_ROOT/src"
                        [[ -d "$src_dir" ]] || src_dir="$PROJECT_ROOT"
                        echo "$venv_bin/$script_name|$src_dir|pip install -e .|Python: $script_name"
                    fi
                fi
            done
        fi
    fi
}

# ============================================================================
# Staleness Check
# ============================================================================

# Compare binary mtime with newest source file mtime
# Returns: "stale" or "fresh"
check_staleness() {
    local binary_path="$1"
    local source_dir="$2"

    [[ -f "$binary_path" ]] || { echo "missing"; return 0; }
    [[ -d "$source_dir" ]] || { echo "unknown"; return 0; }

    local binary_mtime
    binary_mtime=$(stat -c '%Y' "$binary_path" 2>/dev/null || stat -f '%m' "$binary_path" 2>/dev/null || echo "0")

    # Find newest source file (exclude hidden dirs, target/, node_modules/, vendor/, dist/, build/)
    local newest_source_mtime
    newest_source_mtime=$(find "$source_dir" \
        -not -path '*/\.*' \
        -not -path '*/target/*' \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
        -not -path '*/dist/*' \
        -not -path '*/build/*' \
        -not -path '*/__pycache__/*' \
        -type f \
        -newer "$binary_path" \
        2>/dev/null | head -1)

    if [[ -n "$newest_source_mtime" ]]; then
        echo "stale"
    else
        echo "fresh"
    fi
}

# Get the latest modified source file for context
get_latest_source() {
    local source_dir="$1"
    local binary_path="$2"

    find "$source_dir" \
        -not -path '*/\.*' \
        -not -path '*/target/*' \
        -not -path '*/node_modules/*' \
        -not -path '*/vendor/*' \
        -not -path '*/dist/*' \
        -not -path '*/build/*' \
        -not -path '*/__pycache__/*' \
        -type f \
        -newer "$binary_path" \
        2>/dev/null | head -5
}

# ============================================================================
# Main
# ============================================================================

# Collect all detected CLI tools
entries=""
for detector in detect_cargo detect_go detect_node detect_make detect_python; do
    result=$($detector 2>/dev/null || true)
    if [[ -n "$result" ]]; then
        entries="$entries
$result"
    fi
done

# Remove leading empty line
entries=$(echo "$entries" | sed '/^$/d')

# No CLI tools detected - silent exit
if [[ -z "$entries" ]]; then
    exit 0
fi

# Check each binary
stale_entries=""
fresh_count=0
total_count=0

while IFS='|' read -r binary_path source_dir build_cmd label; do
    [[ -z "$binary_path" ]] && continue
    total_count=$((total_count + 1))

    status=$(check_staleness "$binary_path" "$source_dir")

    if [[ "$status" == "stale" ]]; then
        changed_files=$(get_latest_source "$source_dir" "$binary_path")
        changed_count=$(echo "$changed_files" | wc -l | xargs)
        stale_entries="${stale_entries}
- **${label}**: \`$(basename "$binary_path")\` 리빌드 필요
  - 바이너리: \`${binary_path}\`
  - 변경된 소스 ${changed_count}개+
  - 빌드 명령: \`${build_cmd}\`"
    elif [[ "$status" == "missing" ]]; then
        stale_entries="${stale_entries}
- **${label}**: 바이너리 없음 (빌드 필요)
  - 경로: \`${binary_path}\`
  - 빌드 명령: \`${build_cmd}\`"
    else
        fresh_count=$((fresh_count + 1))
    fi
done <<< "$entries"

# Output only if there are stale binaries
if [[ -n "$stale_entries" ]]; then
    stale_count=$((total_count - fresh_count))
    cat << EOF
## CLI Version Check

**Stale 바이너리 ${stale_count}개 감지** (전체 ${total_count}개 중)

소스 코드가 마지막 빌드 이후 변경되었습니다. 리빌드를 권장합니다.
${stale_entries}

> 리빌드 후 최신 바이너리로 작업하세요.
EOF
fi

exit 0
