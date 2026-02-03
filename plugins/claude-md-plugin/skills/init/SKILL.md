---
name: init
aliases: [extract, ext]
description: |
  This skill should be used when the user wants to generate CLAUDE.md files
  for an existing codebase. It analyzes source code structure and creates
  CLAUDE.md documentation for each directory containing source files.

  Trigger keywords:
  - "/init", "/ext"
  - "CLAUDE.md 추출"
  - "소스코드 문서화"
  - "claude-md 생성"
  - "Initialize CLAUDE.md from code"
allowed-tools: [Bash, Read, Task, Skill, AskUserQuestion]
---

# Init Skill

## 목적

기존 소스 코드를 분석하여 CLAUDE.md 초안을 생성합니다.
CLAUDE.md는 해당 디렉토리의 Source of Truth가 되어 코드 재현의 기반이 됩니다.

## 아키텍처

```
User: /init
        │
        ▼
┌─────────────────────────────────────────────┐
│ init SKILL (사용자 진입점)                │
│                                             │
│ 1. Skill("tree-parse") → 대상 목록          │
│ 2. For each directory (leaf-first):         │
│    Task(initializer) 생성                     │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ initializer AGENT (디렉토리별)                 │
│                                             │
│ ┌─ Skill("boundary-resolve") ────────────┐  │
│ │ 바운더리 분석                           │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("code-analyze") ────────────────┐  │
│ │ 코드 분석 (exports, deps, behaviors)    │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ AskUserQuestion ──────────────────────┐  │
│ │ 불명확한 부분 질문                      │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("draft-generate") ──────────────┐  │
│ │ CLAUDE.md 초안 생성                     │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("schema-validate") ─────────────┐  │
│ │ 스키마 검증 (실패시 재시도)             │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## 워크플로우

### 1. Core Engine 빌드 확인

```bash
# CLI가 빌드되어 있는지 확인
CLI_PATH="plugins/claude-md-plugin/core/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd plugins/claude-md-plugin/core && cargo build --release
fi
```

빌드된 CLI가 없으면 먼저 빌드합니다.

### 2. 트리 파싱 (Skill 호출)

```python
# tree-parse Skill 호출
Skill("claude-md-plugin:tree-parse")
# 입력: root_path (기본: 현재 디렉토리)
# 출력: .claude/init-tree.json
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

### 3. 대상 디렉토리 확인

tree.json을 읽고 CLAUDE.md가 필요한 디렉토리 목록을 **depth 내림차순** (leaf-first)으로 정렬하여 사용자에게 보여줍니다.

```
=== CLAUDE.md 생성 대상 ===

다음 디렉토리에 CLAUDE.md를 생성합니다 (leaf-first 순서):
  1. [depth=2] src/auth/ (4 source files)
  2. [depth=2] src/api/ (5 source files)
  3. [depth=1] src/ (2 source files, 3 subdirectories)

제외된 디렉토리: node_modules, target, dist

계속하시겠습니까?
```

**핵심:** 자식 디렉토리 CLAUDE.md가 먼저 생성되어야 부모가 자식의 Purpose를 읽을 수 있습니다.

### 4. Leaf-first 순차 실행

**병렬이 아닌 순차 실행**으로 depth가 깊은 디렉토리(leaf)부터 처리합니다.

```python
# depth 내림차순 정렬 (leaf-first)
sorted_dirs = sorted(needs_claude_md, key=lambda d: -d["depth"])

for dir_info in sorted_dirs:
    # 하위 CLAUDE.md 경로 목록 (이미 생성된 자식들)
    child_claude_mds = find_child_claude_mds(dir_info["path"])

    # initializer Agent 실행
    Task(
        subagent_type="claude-md-plugin:initializer",
        prompt=f"""
대상 디렉토리: {dir_info["path"]}
직접 파일 수: {dir_info["source_file_count"]}
하위 디렉토리 수: {dir_info["subdir_count"]}
자식 CLAUDE.md: {child_claude_mds}

이 디렉토리의 CLAUDE.md를 생성해주세요.
결과 파일: .claude/init-results/{dir_info["path"].replace('/', '-')}.md
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

### 5. 결과 수집 및 검증

각 Agent 실행 완료 즉시 (순차 실행이므로):

1. `.claude/init-results/{dir-name}.md` 결과 파일 확인
2. 검증 통과 시 실제 CLAUDE.md 위치로 복사
3. **중요:** 복사 후 다음 depth의 Agent가 읽을 수 있도록 즉시 배치

```bash
# 검증 성공 시 즉시 배치 (다음 Agent가 읽을 수 있도록)
cp .claude/init-results/src-auth.md src/auth/CLAUDE.md
```

### 6. 최종 보고

```
=== CLAUDE.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md
  ✓ src/auth/CLAUDE.md
  ✓ src/api/CLAUDE.md

검증 결과:
  - 스키마 검증: 3/3 통과
  - 참조 규칙: 3/3 통과

사용자 질문: 5개 응답됨

다음 단계:
  - /context-validate로 재현성 검증 가능
```

## 내부 Skill 목록

이 Skill은 다음 내부 Skill들을 조합합니다:

| Skill | 역할 | 호출 위치 |
|-------|------|----------|
| `tree-parse` | 디렉토리 트리 파싱 | init Skill |
| `boundary-resolve` | 바운더리 분석 | initializer Agent |
| `code-analyze` | 코드 분석 | initializer Agent |
| `draft-generate` | CLAUDE.md 생성 | initializer Agent |
| `schema-validate` | 스키마 검증 | initializer Agent |

내부 Skill은 description에 `(internal)` 표시되어 자동완성에서 숨겨집니다.

## 파일 기반 결과 전달

Agent는 결과를 파일로 저장하고 경로만 반환합니다:

| 컴포넌트 | 결과 파일 경로 |
|---------|--------------|
| tree-parse | `.claude/init-tree.json` |
| initializer | `.claude/init-results/{dir-name}.md` |

이로써 Skill context 폭발을 방지합니다.

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | 에러 메시지 출력, 실패 반환 |
| tree-parse 실패 | CLI 에러 메시지 전달 |
| initializer 실패 | 해당 디렉토리 스킵, 경고 표시 |
| 스키마 검증 실패 | Agent가 최대 5회 재시도 후 경고와 함께 진행 |
