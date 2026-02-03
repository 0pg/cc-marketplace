# Decompile Execution Order Details

## Core Engine 빌드 확인

```bash
# CLI가 빌드되어 있는지 확인
CLI_PATH="plugins/claude-md-plugin/core/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd plugins/claude-md-plugin/core && cargo build --release
fi
```

빌드된 CLI가 없으면 먼저 빌드합니다.

## 트리 파싱 (Skill 호출)

```python
# tree-parse Skill 호출
Skill("claude-md-plugin:tree-parse")
# 입력: root_path (기본: 현재 디렉토리)
# 출력: scratchpad에 저장
```

프로젝트 루트에서 트리를 파싱하여 CLAUDE.md가 필요한 디렉토리 목록을 생성합니다.

**출력 예시** (tree.json):
```json
{
  "root": "/path/to/project",
  "needs_claude_md": [
    {"path": "src", "source_file_count": 2, "subdir_count": 3, "reason": "2 source files and 3 subdirectories", "depth": 1},
    {"path": "src/auth", "source_file_count": 4, "subdir_count": 1, "reason": "4 source files", "depth": 2},
    {"path": "src/api", "source_file_count": 5, "subdir_count": 0, "reason": "5 source files", "depth": 2}
  ],
  "excluded": ["node_modules", "target", "dist"]
}
```

## Leaf-first 순차 실행 로직

**병렬이 아닌 순차 실행**으로 depth가 깊은 디렉토리(leaf)부터 처리합니다.

```python
# depth 내림차순 정렬 (leaf-first)
sorted_dirs = sorted(needs_claude_md, key=lambda d: -d["depth"])

for dir_info in sorted_dirs:
    # 하위 CLAUDE.md 경로 목록 (이미 생성된 자식들)
    child_claude_mds = find_child_claude_mds(dir_info["path"])

    # decompiler Agent 실행
    Task(
        subagent_type="claude-md-plugin:decompiler",
        prompt=f"""
대상 디렉토리: {dir_info["path"]}
직접 파일 수: {dir_info["source_file_count"]}
하위 디렉토리 수: {dir_info["subdir_count"]}
자식 CLAUDE.md: {child_claude_mds}

이 디렉토리의 CLAUDE.md를 생성해주세요.
결과는 scratchpad에 저장하고 경로만 반환해주세요.
""",
        description=f"Extract CLAUDE.md for {dir_info['path']}"
    )

    # 다음 디렉토리 처리 전 완료 대기 (순차 실행)
```

**실행 순서 예시:**
```
1. src/auth/jwt  (depth=3) → CLAUDE.md 생성
2. src/auth      (depth=2) → jwt/CLAUDE.md Purpose 읽기 가능
3. src/api       (depth=2) → CLAUDE.md 생성
4. src           (depth=1) → auth/CLAUDE.md, api/CLAUDE.md Purpose 읽기 가능
```

## 결과 수집 및 검증

각 Agent 실행 완료 즉시 (순차 실행이므로):

1. scratchpad의 결과 파일 확인 (CLAUDE.md + IMPLEMENTS.md)
2. 검증 통과 시 실제 위치로 복사
3. **중요:** 복사 후 다음 depth의 Agent가 읽을 수 있도록 즉시 배치

```bash
# 검증 성공 시 즉시 배치 (다음 Agent가 읽을 수 있도록)
cp {scratchpad_result_file_claude} src/auth/CLAUDE.md
cp {scratchpad_result_file_implements} src/auth/IMPLEMENTS.md
```

## decompiler Agent 아키텍처

```
┌─────────────────────────────────────────────┐
│ decompiler AGENT (디렉토리별)               │
│                                             │
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
│ │ 스키마 검증 (실패시 재시도)             │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```
