---
name: migrate
description: |
  기존 프로젝트를 현재 플러그인 스키마로 수렴시킵니다.
  fix-schema CLI의 converge_schema를 사용하여 섹션 rename/remove/add를 결정론적으로 처리.
  source version 감지 불필요 — target-state-driven migration.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, AskUserQuestion]
---

# /migrate

기존 프로젝트를 현재 플러그인 스키마에 맞게 마이그레이션합니다.
source version을 감지하지 않고, 현재 스키마를 목표 상태로 수렴시킵니다.

## Triggers

- `/migrate`
- `마이그레이션`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | `.` | 프로젝트 루트 경로 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 파일 수집

```
Glob("**/CLAUDE.md", path={project_root_path})
Glob("**/DEVELOPERS.md", path={project_root_path})
```

CLAUDE.md 없으면 종료.

### 2. Dry-run (스키마 수렴 분석)

각 파일에 대해:

```bash
# CLAUDE.md
$CLI_PATH fix-schema --file "$claude_md" --type claude_md --dry-run

# DEVELOPERS.md (있는 경우)
$CLI_PATH fix-schema --file "$developers_md" --type developers_md --dry-run
```

변경 내역(renames, removals, additions)과 경고(conflicts) 수집.

### 3. 레거시 파일 감지

```
Glob("**/IMPLEMENTS.md", path={project_root_path})
Glob(".claude/index.md", path={project_root_path})
Glob("**/compile-context.md", path={project_root_path})
Glob(".claude/tmp/*/bugfix-analysis-*.md", path={project_root_path})
Glob(".claude/tmp/*/compile-session-*.md", path={project_root_path})
Glob(".claude/tmp/*/validate-session-*.md", path={project_root_path})
Glob(".claude/tmp/*/impl-session.md", path={project_root_path})
Glob(".claude/tmp/*/decompile-session-*.md", path={project_root_path})
```

삭제 대상 목록 수집.

### 4. 계획 표시 + 1회 승인

변경 없으면 "마이그레이션 불필요" → 종료.

변경 있으면 계획 표시:
- **스키마 변환**: rename/remove/add 내역 (파일별)
- **파일 정리**: 삭제 대상 레거시 파일 목록
- **충돌 경고**: 둘 다 존재하는 rename 케이스 (수동 해소 필요)

AskUserQuestion으로 1회 승인 요청.

### 5. 실행

```bash
# 스키마 수렴
$CLI_PATH fix-schema --file "$claude_md" --type claude_md
$CLI_PATH fix-schema --file "$developers_md" --type developers_md

# 레거시 파일 삭제
git rm "$legacy_file" 2>/dev/null || rm "$legacy_file"
```

### 6. 충돌 해소 (필요 시)

dry-run에서 conflict warning이 있던 파일에 대해:
- AskUserQuestion: "## {from}과 ## {to}가 모두 존재합니다. (a) 수동 merge (b) /decompile로 재생성"
- 사용자 선택에 따라 처리

### 7. 검증

```bash
$CLI_PATH validate-schema --file "$claude_md" --strict
$CLI_PATH validate-convention --project-root {project_root}
```

검증 실패 시: "/decompile {path}로 재생성을 권장합니다" 안내.

### 8. 결과 보고

```bash
git diff --stat -- "**/CLAUDE.md" "**/DEVELOPERS.md"
```

마이그레이션 결과 + 후속 액션 안내:
- Conventions 부재 시 → `/project-setup` 안내
- Instructions 부재 시 → `/project-setup` 안내

## DO / DON'T

**DO:**
- dry-run 먼저 수행 후 계획 표시 → 1회 승인
- fix-schema CLI에 위임 (결정론적 수렴)
- 충돌 시 사용자 판단 요청

**DON'T:**
- 사용자 승인 없이 파일 삭제
- 파일마다 개별 승인 요청
- source version 감지 로직 작성 (fix-schema가 target-state로 수렴)
