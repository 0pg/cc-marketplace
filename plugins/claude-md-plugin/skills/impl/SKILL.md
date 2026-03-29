---
name: impl
version: 1.0.0
aliases: [define, requirements]
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/impl".
  Analyzes natural language requirements and generates CLAUDE.md without implementing code.
  Follows ATDD principle: specification first, then code generation via /compile.
  Trigger keywords: 요구사항 정의, 스펙 작성, 명세 먼저
user_invocable: true
allowed-tools: [Read, Glob, Write, Task, AskUserQuestion, Bash]
---

# /impl

요구사항(자연어 또는 User Story)을 분석하여 **CLAUDE.md + DEVELOPERS.md**를 생성/업데이트.
**코드 구현 없이** 요구사항 정의만 수행하여 "명세 먼저" 원칙을 따름.

## Triggers

- `/impl`
- `요구사항 정의`
- `스펙 작성`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `requirement` | 예 | - | 요구사항 텍스트 |
| `--path` | 아니오 | `.` | 대상 경로 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 기존 CLAUDE.md 인덱스 생성

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index.json"
```

### 2. 프로젝트 컨벤션 읽기

project root CLAUDE.md의 `## Conventions` 섹션이 있으면 읽기.

### 3. 세션 파일 생성

`${TMP_DIR}impl-session.md`:

```markdown
# Impl Session
type: impl | project_root: {project_root}

## User Requirement
{사용자 요구사항 텍스트}

## Existing Modules Index
{scan-claude-md 결과: path, purpose 쌍}

## Project Conventions
{project root Conventions 또는 "None"}
```

### 4. impl agent 디스패치

```
Task(impl):
  세션 파일: ${TMP_DIR}impl-session.md
  프로젝트 루트: {project_root}

  세션 파일을 읽고 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
```

### 5. 변경사항 표시

```bash
git diff --stat
git diff
```

### 6. 결과

impl agent의 result block을 전달.

## DO / DON'T

**DO:**
- scan-claude-md로 인덱스 생성 후 세션 파일에 포함
- impl agent에게 세션 파일 경로만 전달
- 생성 후 git diff 표시

**DON'T:**
- 코드 생성 (그것은 /compile의 역할)
- CLAUDE.md 직접 수정 (impl agent가 담당)
- impl agent에 인덱스 파일 경로 대신 내용 직접 전달

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| 요구사항 인자 없음 | AskUserQuestion으로 요구사항 수집 |
| impl agent 실패 | 에러 보고 |
