---
name: validate
version: 3.0.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "lint documentation", or uses "/validate".
  Runs schema validation and validator agent for comprehensive drift detection.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task]
---

# /validate

CLAUDE.md와 실제 코드 간의 일치 여부를 검증합니다.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `drift 검사`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 |
| `--strict` | 아니오 | false | DEVELOPERS.md 부재를 error로 취급 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. CLAUDE.md 수집

```
Glob("{path}/**/CLAUDE.md")
```

수집된 파일이 없으면: "대상 경로에 CLAUDE.md가 없습니다." → 종료.

### 2. 스키마 사전 검증

각 CLAUDE.md에 대해 스키마 검증:

```bash
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}schema-${dir_safe}.json"
```

실패 시 auto-fix 시도:
```bash
$CLI_PATH fix-schema --file "$claude_md"
# 재검증
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}schema-${dir_safe}.json"
```

auto-fix 후에도 실패하면 해당 모듈을 스키마 오류로 보고하고 drift 검증 대상에서 제외.

### 3. Drift 검증 (validator agent)

스키마 통과한 CLAUDE.md 디렉토리를 배치로 나누어 `Task(validator)` 실행.

**배치 규칙**: 최대 3개 디렉토리를 병렬 처리.

각 배치:
```
Task(validator): "검증 대상: {directory}"
```

validator agent가 검증하는 4개 drift 카테고리:
1. **Requirements Drift** — 코드가 명시된 요구사항을 위반/미적용
2. **Convention Drift** — 코딩 규칙 위반
3. **DEVELOPERS.md Drift (INV-3)** — DEVELOPERS.md 부재, Constraints/Technical Context 불일치
4. **Boundary Violations (INV-1)** — 트리 구조 의존성 위반

결과를 `${TMP_DIR}validate-progress.jsonl`에 누적:
```bash
echo '{"directory":"{dir}","issues":{n},"status":"{status}"}' >> "${TMP_DIR}validate-progress.jsonl"
```

### 4. 통합 보고서 생성

모든 validator 결과 파일을 읽어 통합 보고서를 생성합니다:

```markdown
# Validation Report

## 요약

| 지표 | 값 |
|------|-----|
| 검증 대상 | {total}개 모듈 |
| 스키마 통과 | {schema_pass}/{total} |
| Drift 없음 | {clean}/{schema_pass} |
| 총 이슈 | {total_issues}개 |

## 스키마 오류

| 모듈 | 오류 |
|------|------|
| {path} | {error_message} |

## Drift 이슈

### {module_path}

| 카테고리 | 유형 | 설명 | 신뢰도 |
|----------|------|------|--------|
| Requirements | VIOLATED | {description} | MEDIUM |
| DEVELOPERS.md | MISSING | DEVELOPERS.md 부재 | HIGH |

## 추천 액션

1. 스키마 오류 수정: {paths}
2. Drift 해소: `/resolve`
3. DEVELOPERS.md 생성: `/decompile {paths}`
```

보고서를 `${TMP_DIR}validate-report.md`에 저장하고 사용자에게 출력합니다.

## DO / DON'T

**DO:**
- 스키마 검증 → auto-fix → drift 검증 순서
- validator agent 병렬 실행 (최대 3개)
- 결과를 파일로 저장
- `/resolve` 연계 안내

**DON'T:**
- CLAUDE.md나 소스코드 수정 (검증만)
- 스키마 실패한 모듈에 drift 검증 수행
- 사용자에게 각 validator의 진행 상황 중계

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| validator agent 실패 | 해당 모듈 스킵, 경고 |
| auto-fix 실패 | 스키마 오류로 보고 |

## Examples

<example>
<user_request>/validate src</user_request>
<assistant_response>
CLAUDE.md 4개 수집 완료.

스키마 검증: 4/4 통과
Drift 검증 진행 중... (배치 1/2)

Validation Report
=================

| 지표 | 값 |
|------|-----|
| 검증 대상 | 4개 모듈 |
| 스키마 통과 | 4/4 |
| Drift 없음 | 2/4 |
| 총 이슈 | 5개 |

Drift 이슈:
  src/auth: Requirements VIOLATED (1), Convention MISSING_SUBSECTION (1)
  src/legacy: DEVELOPERS.md MISSING (1), Boundary VIOLATED (2)

추천: `/resolve` 로 drift 해소
</assistant_response>
</example>
