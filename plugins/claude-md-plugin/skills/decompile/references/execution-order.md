# Decompile Execution Order Details

## Core Engine 빌드 확인

```bash
# CLI가 빌드되어 있는지 확인
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi
```

빌드된 CLI가 없으면 먼저 빌드합니다.

## 트리 파싱 (Skill 호출)

`tree-parse` 스킬을 호출합니다. 입력은 `root_path` (기본: 현재 디렉토리)이며, 결과는 파일로 저장됩니다.

프로젝트 루트에서 트리를 파싱하여 CLAUDE.md가 필요한 디렉토리 목록을 생성합니다.

## tree.json에서 필요한 정보만 추출 (jq)

tree.json 전체를 Read하지 않고, jq로 필요한 필드만 추출하여 main context 적재를 최소화합니다.

```bash
# 디렉토리 수 확인
Bash("jq '.needs_claude_md | length' {tree_file}")

# depth-path 목록 추출 (leaf-first 정렬)
Bash("jq -r '.needs_claude_md | sort_by(-.depth) | .[] | \"\\(.depth) \\(.path)\"' {tree_file}")
```

**출력 예시:**
```
3 src/auth/jwt
2 src/auth
2 src/api
1 src
```

## Foreground + 압축 응답 실행 로직

**Foreground Task + 압축 응답 방식으로 순차 실행.** depth가 깊은 디렉토리(leaf)부터 처리합니다.

### 실행 절차

1. `needs_claude_md`를 jq로 depth 내림차순 정렬하여 "depth path" 목록을 추출합니다.
2. 각 디렉토리에 대해 순차적으로 다음을 수행합니다:

   a. 자식 CLAUDE.md 경로를 수집합니다 (이미 생성된 직접 자식만):
   ```bash
   Bash("jq -r '.needs_claude_md[] | select(.path | startswith(\"{path}/\")) | select(.path | ltrimstr(\"{path}/\") | contains(\"/\") | not) | .path + \"/CLAUDE.md\"' {tree_file}")
   ```

   b. `decompiler` agent를 **foreground Task**로 호출합니다:
   ```
   Task(decompiler, prompt="대상: {path}  tree: {tree_file}\n자식 CLAUDE.md: {children_list}")
   ```
   → agent가 result block만 반환 (~5줄)

   c. result block에서 status 확인 후 다음 디렉토리 처리 (실패 시 skip + 경고)

### Context 절약 효과

| 항목 | Before (원본) | After (foreground+압축) |
|------|-------------|----------------------|
| Task prompt | ~10줄 | ~2줄 |
| Task result | ~15줄 | ~5줄 (result block) |
| **합계** | ~25줄 | ~7줄 |

### 실행 순서 예시

```
1. src/auth/jwt  (depth=3) → Task(decompiler) → success
2. src/auth      (depth=2) → Task(decompiler) → success (jwt/CLAUDE.md 읽기 가능)
3. src/api       (depth=2) → Task(decompiler) → success
4. src           (depth=1) → Task(decompiler) → success (auth/, api/ CLAUDE.md 읽기 가능)
```

## 결과 수집 및 검증

decompiler agent가 CLAUDE.md + IMPLEMENTS.md를 **대상 디렉토리에 직접 Write**.
scratchpad 중간 저장 및 Read+Write 복사 과정 없음 (context 적재 방지).

각 Agent 실행 완료 즉시 (순차 실행이므로):

1. decompiler agent가 대상 디렉토리에 직접 생성 완료
2. **중요:** 다음 depth의 Agent가 자식 CLAUDE.md를 읽을 수 있도록 즉시 배치됨

## decompiler Agent 아키텍처

```
┌─────────────────────────────────────────────┐
│ decompiler AGENT (디렉토리별)                 │
│                                             │
│ ┌─ Bash(jq) ───────────────────────────────┐│
│ │ tree.json에서 디렉토리 정보 조회          ││
│ │ (source_file_count, subdir_count)         ││
│ └──────────────────┬───────────────────────┘│
│                    ▼                        │
│ ┌─ Skill("boundary-resolve") ────────────┐  │
│ │ 바운더리 분석                           │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("code-analyze") ────────────────┐  │
│ │ 코드 분석 (exports, deps, behaviors)    │  │
│ │ + 알고리즘, 상수, 에러처리, 상태 분석   │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ AskUserQuestion ──────────────────────┐  │
│ │ 불명확한 부분 질문                      │  │
│ │ (Domain Context, Implementation 배경)   │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ CLAUDE.md + IMPLEMENTS.md 생성 ───────┐  │
│ │ CLAUDE.md: WHAT 초안 생성               │  │
│ │ IMPLEMENTS.md: HOW 전체 섹션 생성       │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("schema-validate") ─────────────┐  │
│ │ 스키마 검증 (실패시 경고)               │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Result block만 출력 ─────────────────┐   │
│ │ ---decompiler-result---                │   │
│ │ status: success                        │   │
│ │ target_dir: {path}                     │   │
│ │ validation: passed                     │   │
│ │ ---end-decompiler-result---            │   │
│ └────────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```
