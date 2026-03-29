---
name: decompile
version: 1.0.0
aliases: [decom, extract, document]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", "extract documentation from source",
  or uses "/decompile" or "/decom".
  Analyzes existing source code and creates CLAUDE.md + DEVELOPERS.md documentation for each directory.
  Uses parse-tree CLI for directory discovery, then runs decompiler agent per directory in leaf-first order.
  Trigger keywords: 디컴파일, 코드에서 문서 추출, 기존 코드 문서화
user_invocable: true
allowed-tools: [Bash, Read, Write, Glob, Task, AskUserQuestion]
---

# /decompile

소스코드를 분석하여 CLAUDE.md + DEVELOPERS.md를 추출합니다.

## Triggers

- `/decompile`
- `코드에서 문서 추출`
- `기존 코드 문서화`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 0.5. `## Instructions` 존재 확인 (fallback)

project root CLAUDE.md에 `## Instructions` 섹션이 없으면 자동 생성:

```markdown
## Instructions

- CLAUDE.md is the SSOT. Source code is a derived artifact generated from CLAUDE.md.
- When code disagrees with CLAUDE.md, regenerate code via /compile (not modify docs).
- To change requirements, update CLAUDE.md first, then code follows.
- Derive tests from DEVELOPERS.md Constraints.
- 소스코드는 /compile로 생성. Write tool로 직접 소스 파일 생성 금지.
- 완료 선언 전 /validate --strict 실행 필수.
```

이미 존재하면 skip. `/project-setup`이 primary entry point.

### 1. 디렉토리 트리 파싱 (inline)

tree-parse를 별도 스킬이 아닌 CLI 직접 호출로 처리:

```bash
$CLI_PATH parse-tree --root {path} --output .claude/extract-tree.json
```

### 2. 실행 순서 결정

`needs_claude_md` 배열을 depth DESC 정렬 (leaf-first).

### 3. 세션 파일 생성 + decompiler 실행

정렬된 순서(leaf-first)로 각 디렉토리에 대해:

1. 자식 CLAUDE.md 목록 생성 (하위 디렉토리 중 이미 CLAUDE.md가 생성된 것)
2. 프로젝트 컨벤션 읽기 (있는 경우)
3. 세션 파일 Write → `${TMP_DIR}decompile-session-{dir-safe}.md`:

```markdown
# Decompile Task: {path}
type: decompile | target: {path}

## Tree Info
source_file_count: {n}
subdir_count: {n}
depth: {n}

## Children CLAUDE.md
{이미 생성된 자식 CLAUDE.md 경로 목록, 없으면 "None"}

## Project Conventions
{project root Conventions 또는 "None"}
```

4. `Task(decompiler)` 호출:
```
세션 파일: ${TMP_DIR}decompile-session-{dir-safe}.md
대상: {path}
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

같은 depth의 독립 디렉토리는 병렬 실행 가능 (최대 3개).

decompiler 결과 status 확인:
- `success`: 다음 모듈로
- `failed_with_warnings`: 경고 수집, 다음 모듈로

### 4. 변경사항 표시

```bash
git diff --stat
```

### 5. 결과

```
---decompile-result---
status: success | partial | failed
total: {n}
generated: {n}
failed: {n}
---end-decompile-result---
```

## DO / DON'T

**DO:**
- leaf-first 순서 준수
- 자식 CLAUDE.md 목록을 세션 파일에 포함
- decompiler agent에게 세션 파일로 위임

**DON'T:**
- 코드 수정 (이것은 추출 작업)
- 생성된 CLAUDE.md에 소스코드 직접 복사
- decompiler agent에 raw tree.json 전체 전달 (필요한 정보만 세션 파일에)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| 소스 파일 없는 디렉토리 | skip |
| decompiler agent 실패 (단일 모듈) | 경고, 나머지 계속 |
