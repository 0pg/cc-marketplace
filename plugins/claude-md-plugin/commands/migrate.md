---
name: migrate
description: |
  claude-md-plugin 버전 업그레이드 시 기존 프로젝트를 새 버전에 맞게 마이그레이션합니다.
  v6→v7 스키마 전환, 레거시 IMPLEMENTS.md 정리, 스키마 누락 섹션 추가, 조건부 정리,
  v9→v10 전환 (제거된 커맨드 안내, 세션 파일 정리).
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, Skill, AskUserQuestion]
---

# /migrate

기존 프로젝트를 현재 플러그인 버전에 맞게 마이그레이션합니다.

## Triggers

- `/migrate`
- `마이그레이션`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | `.` | 프로젝트 루트 경로 |

## 마이그레이션 유형

다섯 가지 마이그레이션을 자동 감지하여 처리합니다:

1. **레거시 정리**: IMPLEMENTS.md 삭제 (v2.x → v3.0+)
2. **스키마 업그레이드**: CLAUDE.md 누락 필수 섹션 추가 (v3.x → v4.0+)
3. **조건부 정리**: 불필요한 conditional "None" 섹션 제거 (v4.x → v5.0+)
4. **v6→v7 전환**: CLAUDE.md Constraints→Requirements, DEVELOPERS.md 4섹션 스키마 (v6.x → v7.0+)
5. **v9→v10 전환**: 제거된 커맨드 안내, 세션 파일 정리 (v9.x → v10.0+)

## Workflow

### 1. 사전 확인

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

문서 파일 수집:
```
Glob("**/CLAUDE.md", path={project_root_path})
Glob("**/IMPLEMENTS.md", path={project_root_path})
Glob("**/DEVELOPERS.md", path={project_root_path})
```

CLAUDE.md 없으면 종료.

### 2. 마이그레이션 유형 자동 감지

| 감지 조건 | 유형 |
|-----------|------|
| IMPLEMENTS.md 존재 | LEGACY_CLEANUP |
| CLAUDE.md에 `## Constraints` 존재 | V6_TO_V7 |
| CLAUDE.md 스키마 검증 FAIL | SCHEMA_UPGRADE |
| 불필요 conditional "None" 섹션 | CONDITIONAL_CLEANUP |
| v9 세션 파일 잔존 또는 /dev, /convention-update 참조 | V9_TO_V10 |
| 해당 없음 | UP_TO_DATE |

#### V9_TO_V10 감지 로직

```bash
# v9 세션 파일 잔존 확인
ls .claude/tmp/*/bugfix-analysis-*.md 2>/dev/null
ls .claude/tmp/*/compile-session-*.md 2>/dev/null

# CLAUDE.md Instructions에 v9 전용 참조 확인
grep -r "convention-update\|/dev " {project_root}/CLAUDE.md 2>/dev/null
```

UP_TO_DATE인 경우 "마이그레이션 불필요" 메시지 출력 후 종료.

### 3. 마이그레이션 계획 표시 + 승인

감지된 항목을 표시하고 1회 승인 요청.

### 4. 레거시 정리 (IMPLEMENTS.md 삭제)

LEGACY_CLEANUP 감지 시:
```bash
git rm "$impl_md" 2>/dev/null || rm "$impl_md"
```

### 5. v6→v7 전환

V6_TO_V7 감지 시:
- CLAUDE.md: `## Constraints` → `## Requirements`
- DEVELOPERS.md: `## Domain Context` → `## Technical Context`, `## Invariants` → `## Constraints`, `## File Map` 삭제
- `.claude/index.md`, `compile-context.md` 삭제
- 선택적 LLM 보조 분류 (PM-level vs developer-level 분류)

### 6. 스키마 업그레이드

SCHEMA_UPGRADE 감지 시:
```bash
$CLI_PATH fix-schema --file "$claude_md"
```

### 6.5. 조건부 정리

CONDITIONAL_CLEANUP 감지 시:
- 불필요 conditional "None" 섹션 제거
- Decision Log 언어 정규화

### 7. v9→v10 전환

V9_TO_V10 감지 시:

#### 7.1. 제거된 커맨드 안내

```
v10 변경 사항:
  - /dev 커맨드 제거 → 스킬 직접 호출 (/impl, /compile 등) 또는 superpowers 라우팅
  - /convention-update 커맨드 제거 → /project-setup --update로 통합
```

#### 7.2. 세션 파일 정리

```bash
# v9 잔존 세션 파일 정리
rm -f .claude/tmp/*/bugfix-analysis-*.md
rm -f .claude/tmp/*/compile-session-*.md
rm -f .claude/tmp/*/validate-session-*.md
rm -f .claude/tmp/*/impl-session.md
rm -f .claude/tmp/*/decompile-session-*.md
```

#### 7.3. Instructions 업데이트 (선택적)

CLAUDE.md Instructions에 v9 전용 참조가 있으면 제거 제안:
```
AskUserQuestion: "Instructions에서 /dev, /convention-update 참조를 제거하시겠습니까?"
```

### 8. 재검증 + Diff 표시

```bash
$CLI_PATH validate-schema --file "$claude_md" --strict
git diff -- "**/CLAUDE.md" "**/DEVELOPERS.md"
```

### 9. Conventions 부재 감지

```bash
$CLI_PATH validate-convention --project-root {project_root}
```

실패 시 `/project-setup` 실행 제안.

### 10. 결과 보고

마이그레이션 결과 + 후속 액션 안내.

## DO / DON'T

**DO:**
- 자동 감지 후 계획 표시 → 1회 승인
- v6→v7은 안전한 리네임 (내용 보존)
- v9→v10은 정리 + 안내 (문서 스키마 변경 없음)

**DON'T:**
- 사용자 승인 없이 파일 삭제
- 파일마다 개별 승인 요청
