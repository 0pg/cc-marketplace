---
name: validate
version: 3.0.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "lint documentation", or uses "/validate".
  Runs deterministic CLI validation (schema, convention structure, boundary) and semantic validator agent for comprehensive drift detection.
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

### 2. Deterministic 검증 (CLI only)

#### 2a. 스키마 검증 + auto-fix

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

auto-fix 후에도 실패하면 해당 모듈을 스키마 오류로 보고하고 Phase 3 대상에서 제외.

#### 2b. Convention 구조 검증

스키마 통과한 모듈에 대해 Convention 섹션 구조를 검증:

```bash
$CLI_PATH validate-convention --project-root {project_root} --output "${TMP_DIR}convention-result.json"
```

결과에서 `MISSING_CONVENTION`, `MISSING_SUBSECTION` 이슈를 수집합니다.
Convention 구조 이슈는 보고만 하며 Phase 3 진행을 차단하지 않습니다.

#### 2c. Boundary 검증

스키마 통과한 각 디렉토리에 대해 트리 구조 의존성을 검증:

```bash
$CLI_PATH resolve-boundary --path {dir} --claude-md {dir}/CLAUDE.md --output "${TMP_DIR}boundary-${dir_safe}.json"
```

결과의 `violations` 배열에서 `PARENT_REFERENCE`, `SIBLING_REFERENCE` 이슈를 수집합니다.

#### Phase 2 결과 저장

모든 Deterministic 검증 결과를 저장:
```bash
# ${TMP_DIR}deterministic-results.json에 schema/convention/boundary 결과 통합
```

**Gate 규칙**: Schema 실패 모듈만 Phase 3 스킵. Convention/Boundary 결과는 보고만.

### 3. Semantic 검증 (validator agent)

스키마 통과한 CLAUDE.md 디렉토리를 배치로 나누어 `Task(validator)` 실행.

**배치 규칙**: 최대 3개 디렉토리를 병렬 처리.

각 배치:
```
Task(validator): "검증 대상: {directory}"
```

validator agent가 검증하는 3개 semantic drift 카테고리:
1. **Requirements Drift** — 코드가 명시된 요구사항을 위반/미적용
2. **Convention CODE_VIOLATION** — 코드가 Convention 규칙을 위반 (샘플 기반)
3. **DEVELOPERS.md Drift** — DEVELOPERS.md 부재, Constraints/Technical Context 불일치

결과를 `${TMP_DIR}validate-progress.jsonl`에 누적:
```bash
echo '{"directory":"{dir}","issues":{n},"status":"{status}"}' >> "${TMP_DIR}validate-progress.jsonl"
```

### 4. 통합 보고서 생성

Phase 2 (Deterministic) 결과와 Phase 3 (Semantic) 결과를 병합하여 단일 보고서를 생성합니다:

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
| Requirements | STALE | {description} | LOW |
| Convention | MISSING_CONVENTION | project_root에 Conventions 없음 | HIGH |
| Convention | MISSING_SUBSECTION | 필수 서브섹션 없음 | HIGH |
| Convention | CODE_VIOLATION | {description} | MEDIUM |
| DEVELOPERS.md | MISSING | DEVELOPERS.md 부재 | HIGH |
| DEVELOPERS.md | CONSTRAINTS_STALE | {description} | MEDIUM |
| Boundary | PARENT_REFERENCE | 부모 참조 위반 | HIGH |
| Boundary | SIBLING_REFERENCE | 형제 참조 위반 | HIGH |

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
  src/legacy: DEVELOPERS.md MISSING (1), Boundary PARENT_REFERENCE (2)

추천: `/resolve` 로 drift 해소
</assistant_response>
</example>
