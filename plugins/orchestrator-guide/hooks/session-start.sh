#!/bin/bash
# Session Start Hook - orchestrator-guide plugin
# Injects orchestrator context on session start, clear, resume, and compaction
# Includes task continuation detection for incomplete work from previous sessions
#
# This script is designed to be project-agnostic.
# Path patterns and module structure are detected dynamically or from project config.

set -e

# Read JSON input from stdin
input=$(cat)
source=$(echo "$input" | jq -r '.source // "startup"')
session_id=$(echo "$input" | jq -r '.session_id // "unknown"')

# Project root detection
# CLAUDE_PROJECT_DIR is the absolute path to project root, always use it when available
PROJECT_ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"

# ============================================================================
# Configuration Discovery
# ============================================================================

# Try to find module path pattern from project config or use common patterns
# Supports: crates/, packages/, modules/, libs/, src/
find_module_pattern() {
    # Check for common module directory patterns
    for pattern in "crates" "packages" "modules" "libs"; do
        if [[ -d "$PROJECT_ROOT/$pattern" ]]; then
            echo "$pattern"
            return
        fi
    done
    # Fallback: no module pattern (flat structure)
    echo ""
}

MODULE_PATTERN=$(find_module_pattern)

# ============================================================================
# Module/Crate Detection Functions
# ============================================================================

# Function to find target module from current directory
find_target_module() {
    local module_pattern="$1"

    # If no module pattern, return empty
    if [[ -z "$module_pattern" ]]; then
        echo ""
        return
    fi

    # Check if we're in a specific module directory
    if [[ "$PWD" == *"/$module_pattern/"* ]]; then
        echo "$PWD" | sed "s|.*/$module_pattern/||" | cut -d'/' -f1
    else
        echo ""
    fi
}

# Function to resolve task.md path correctly
# Handles both: running from project root AND from within module directory
resolve_task_file() {
    local module=$1
    local module_pattern="$2"

    # If no module specified, try to find task.md in project root
    if [[ -z "$module" ]]; then
        if [[ -f "$PROJECT_ROOT/spec/task.md" ]]; then
            echo "$PROJECT_ROOT/spec/task.md"
            return
        fi
        echo ""
        return
    fi

    # If no module pattern, can't resolve module-specific task
    if [[ -z "$module_pattern" ]]; then
        echo ""
        return
    fi

    # Try project-root-relative path first (standard location)
    local project_path="$PROJECT_ROOT/$module_pattern/$module/spec/task.md"
    if [[ -f "$project_path" ]]; then
        echo "$project_path"
        return
    fi

    # If we're inside the module directory, try relative path
    if [[ "$PWD" == *"/$module_pattern/$module"* ]]; then
        # Find spec/task.md relative to current module
        local module_root="${PWD%%/$module_pattern/$module*}/$module_pattern/$module"
        local module_path="$module_root/spec/task.md"
        if [[ -f "$module_path" ]]; then
            echo "$module_path"
            return
        fi

        # Also try PWD-relative if we're exactly in the module root
        if [[ -f "$PWD/spec/task.md" ]]; then
            echo "$PWD/spec/task.md"
            return
        fi
    fi

    echo ""
}

# Function to find all task.md files in project
find_all_task_files() {
    # First check for root-level task.md
    if [[ -f "$PROJECT_ROOT/spec/task.md" ]]; then
        echo "$PROJECT_ROOT/spec/task.md"
    fi

    # If module pattern exists, find module-specific task files
    if [[ -n "$MODULE_PATTERN" ]]; then
        find "$PROJECT_ROOT/$MODULE_PATTERN" -path "*/spec/task.md" -type f 2>/dev/null || true
    fi
}

# Function to count incomplete tasks in a task file
# Sets global variables: _pending_count, _in_progress_count
count_incomplete_tasks() {
    local task_file=$1
    _pending_count=0
    _in_progress_count=0

    if [[ ! -f "$task_file" ]]; then
        return
    fi

    # Count pending [ ] tasks (tr -d removes any whitespace/newlines)
    _pending_count=$(grep -cE '^\s*-\s*\[\s*\]' "$task_file" 2>/dev/null | tr -d '[:space:]' || true)
    [[ -z "$_pending_count" ]] && _pending_count=0

    # Count in-progress [~] tasks
    _in_progress_count=$(grep -cE '^\s*-\s*\[~\]' "$task_file" 2>/dev/null | tr -d '[:space:]' || true)
    [[ -z "$_in_progress_count" ]] && _in_progress_count=0
}

# Function to read task.md summary
read_task_summary() {
    local module=$1
    local task_file=$(resolve_task_file "$module" "$MODULE_PATTERN")

    if [[ -n "$task_file" && -f "$task_file" ]]; then
        # Task patterns:
        # - [x] completed task
        # - [ ] pending task (next to do)
        # - [~] in-progress task
        # Count completed and pending tasks
        local completed=$(grep -cE '^\s*-\s*\[x\]' "$task_file" 2>/dev/null || echo "0")
        local pending=$(grep -cE '^\s*-\s*\[\s*\]' "$task_file" 2>/dev/null || echo "0")
        local in_progress=$(grep -cE '^\s*-\s*\[~\]' "$task_file" 2>/dev/null || echo "0")
        local next_task=$(grep -E '^\s*-\s*\[\s*\]' "$task_file" | head -1 | sed 's/.*\[ \]\s*//' || echo "없음")

        # Trim whitespace from next_task
        next_task=$(echo "$next_task" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        [[ -z "$next_task" ]] && next_task="없음"

        if [[ "$in_progress" -gt 0 ]]; then
            echo "완료: $completed | 진행중: $in_progress | 대기: $pending | 다음: $next_task"
        else
            echo "완료: $completed | 대기: $pending | 다음: $next_task"
        fi
    else
        echo "task.md 없음"
    fi
}

# Function to check orchestrator state
check_orchestrator_state() {
    local state_file="$PROJECT_ROOT/.claude/orchestrator-state.json"

    if [[ -f "$state_file" ]]; then
        local active=$(jq -r '.active_tasks | length // 0' "$state_file" 2>/dev/null || echo "0")
        local last_status=$(jq -r '.completed_arbs[-1].status // "없음"' "$state_file" 2>/dev/null || echo "없음")
        echo "활성 작업: $active | 마지막 ARB: $last_status"
    else
        echo "상태 파일 없음"
    fi
}

# Function to detect incomplete tasks from previous sessions
# Returns continuation guidance if incomplete work is detected
detect_incomplete_tasks() {
    local total_pending=0
    local total_in_progress=0
    local modules_with_work=""
    local state_file="$PROJECT_ROOT/.claude/orchestrator-state.json"

    # Check orchestrator-state.json for active tasks or incomplete ARBs
    local state_active=0
    local state_incomplete_arbs=0
    if [[ -f "$state_file" ]]; then
        state_active=$(jq -r '.active_tasks | length // 0' "$state_file" 2>/dev/null || echo "0")
        # Count ARBs with status other than "success" or "completed"
        state_incomplete_arbs=$(jq -r '[.completed_arbs[]? | select(.status != "success" and .status != "completed")] | length // 0' "$state_file" 2>/dev/null || echo "0")
    fi

    # Scan all task.md files for incomplete tasks
    while IFS= read -r task_file; do
        [[ -z "$task_file" ]] && continue

        # count_incomplete_tasks sets _pending_count and _in_progress_count
        count_incomplete_tasks "$task_file"

        if [[ "$_pending_count" -gt 0 || "$_in_progress_count" -gt 0 ]]; then
            # Extract module name from path (handle both root and module-based)
            local module_name
            if [[ "$task_file" == *"/$MODULE_PATTERN/"* && -n "$MODULE_PATTERN" ]]; then
                module_name=$(echo "$task_file" | sed "s|.*/$MODULE_PATTERN/||" | cut -d'/' -f1)
            else
                module_name="root"
            fi

            total_pending=$((total_pending + _pending_count))
            total_in_progress=$((total_in_progress + _in_progress_count))

            if [[ "$_in_progress_count" -gt 0 ]]; then
                modules_with_work="${modules_with_work}${module_name}(진행중:${_in_progress_count}) "
            elif [[ "$_pending_count" -gt 0 ]]; then
                modules_with_work="${modules_with_work}${module_name}(대기:${_pending_count}) "
            fi
        fi
    done < <(find_all_task_files)

    # Build continuation message if incomplete work detected
    local has_incomplete=false
    local continuation_msg=""

    # Priority 1: In-progress tasks from previous session (highest priority - interrupted work)
    if [[ "$total_in_progress" -gt 0 ]]; then
        has_incomplete=true
        continuation_msg="### 미완료 작업 감지

**진행중 작업 ${total_in_progress}개 발견**: 이전 세션에서 중단된 작업이 있습니다.
- 대상: ${modules_with_work}

\`/orchestrator\`로 작업 재개를 권장합니다."
    # Priority 2: Active tasks in orchestrator state
    elif [[ "$state_active" -gt 0 ]]; then
        has_incomplete=true
        continuation_msg="### 미완료 작업 감지

**활성 작업 ${state_active}개**: orchestrator-state.json에 미완료 작업이 있습니다.

\`/orchestrator\`로 작업 재개를 권장합니다."
    # Priority 3: Incomplete ARBs (partial, blocked, failed)
    elif [[ "$state_incomplete_arbs" -gt 0 ]]; then
        has_incomplete=true
        continuation_msg="### 미완료 ARB 감지

**미완료 ARB ${state_incomplete_arbs}개**: 이전 작업 결과 중 실패/차단된 항목이 있습니다.

\`/orchestrator\`로 상태 확인을 권장합니다."
    # Priority 4: Pending tasks in task.md (optional work available)
    elif [[ "$total_pending" -gt 0 ]]; then
        # Only show if pending tasks are relatively few (< 10) - likely intentional work items
        # Large numbers suggest initial project setup, not continuation
        if [[ "$total_pending" -lt 10 ]]; then
            has_incomplete=true
            continuation_msg="### 대기 작업 있음

**대기 작업 ${total_pending}개**: task.md에 미완료 작업이 있습니다.
- 대상: ${modules_with_work}

작업 계속 시 \`/orchestrator\`를 사용하세요."
        fi
    fi

    if [[ "$has_incomplete" == "true" ]]; then
        echo "$continuation_msg"
    fi
}

# ============================================================================
# Post-Compaction Core Principles Injection
# ============================================================================

# Inject core orchestrator principles after compaction to restore lost context
inject_core_principles() {
    cat << 'PRINCIPLES'
## Orchestrator Core Principles (Post-Compaction Refresh)

### 1. 5요소 위임 프로토콜
에이전트에 작업 위임 시 반드시 5요소를 포함:
- **GOAL**: 달성할 목표
- **CONTEXT**: 필요한 맥락
- **CONSTRAINTS**: 제약 조건
- **SUCCESS**: 성공 기준
- **HANDOFF**: 후속 처리

### 2. 에이전트 불신 원칙
- 에이전트 출력은 항상 검증
- ARB 형식으로 구조화된 결과 요구
- 실패 시 재시도 또는 대안 탐색

### 3. 병렬 실행 전략
- 독립적 작업은 병렬로 위임
- 의존성 있는 작업은 순차 실행

### 4. ARB 형식 (Agent Result Block)
```yaml
---agent-result---
status: success | partial | blocked | failed
agent: {agent_name}
task_ref: {task_id}
files:
  created: []
  modified: []
verification:
  tests: pass | fail
  lint: pass | fail
issues: []
followup: []
---end-agent-result---
```
PRINCIPLES
}

# ============================================================================
# Main Execution
# ============================================================================

# Determine target module
target_module=$(find_target_module "$MODULE_PATTERN")

# Detect incomplete tasks from previous sessions
continuation_guidance=$(detect_incomplete_tasks)

# Build context message
context_msg="## Orchestrator Session Context

**Session**: $session_id
**Source**: $source
**Module**: ${target_module:-"미지정 (질문 필요)"}

### 상태
- $(check_orchestrator_state)
- $(read_task_summary "$target_module")"

# Add continuation guidance if there are incomplete tasks
if [[ -n "$continuation_guidance" ]]; then
    context_msg="$context_msg

$continuation_guidance"
fi

# Add protocol reference
context_msg="$context_msg

### 위임 프로토콜 (5요소)
GOAL → CONTEXT → CONSTRAINTS → SUCCESS → HANDOFF

---
"

# Inject core principles after compaction (to restore lost detailed instructions)
if [[ "$source" == "compact" ]]; then
    context_msg="$context_msg
$(inject_core_principles)"
fi

# Output context (will be added to Claude's context)
echo "$context_msg"

# Log session start (optional)
# echo "[$(date)] Session $source: $session_id" >> "$PROJECT_ROOT/.claude/session.log"

exit 0
