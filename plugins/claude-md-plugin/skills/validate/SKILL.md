---
name: validate
version: 1.1.0
aliases: [check, verify, lint]
trigger:
  - /validate
  - 문서 검증
  - check documentation
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify spec matches implementation", or uses "/validate". Runs drift-validator and export-validator in parallel.

  <example>
  <user_request>/validate</user_request>
  <assistant_response>
  CLAUDE.md 검증 보고서
  =====================

  요약
  ----
  검증 대상: 3개 디렉토리
  - 양호: 1개
  - 개선 권장: 1개
  - 개선 필요: 1개
  </assistant_response>
  </example>

  <example>
  <user_request>/validate src/</user_request>
  <assistant_response>
  CLAUDE.md 검증 보고서
  =====================

  상세 결과
  ---------
  src/auth (양호)
    Drift: 0개 이슈
    Export 커버리지: 95% (18/19 예측 성공)
  </assistant_response>
  </example>
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill]
---

# /validate

CLAUDE.md 문서의 품질과 코드 일치 여부를 검증합니다.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `문서 검증`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 검증 대상 경로 (디렉토리 또는 파일) |

## Workflow

### 0. Completeness 검증 (audit)

CLI로 CLAUDE.md 완성도를 먼저 검사합니다:

```bash
./plugins/claude-md-plugin/core/target/release/claude-md-core audit \
  --root {path} \
  --only-issues \
  --output .claude/tmp/{session-id}-audit-result.json
```

**결과 상태:**
| 상태 | 의미 |
|------|------|
| `missing` | CON-1 충족하지만 CLAUDE.md 없음 → **생성 필요** |
| `unexpected` | CON-1 미충족인데 CLAUDE.md 있음 → 삭제 검토 |

### 1. 대상 수집

Glob으로 대상 경로의 모든 CLAUDE.md 수집:

```
Glob("**/CLAUDE.md", path={path})
```

### 2. 병렬 검증 실행

각 CLAUDE.md에 대해 두 validator를 **단일 메시지에서 병렬로 Task 호출**:

```
# 반드시 단일 메시지에서 모든 Task를 호출하여 병렬 실행
For each claude_md_file:
  directory = dirname(claude_md_file)

  Task(drift-validator, prompt="검증 대상: {directory}")
  Task(export-validator, prompt="검증 대상: {directory}")
```

**중요**: 성능 최적화를 위해 모든 Task를 하나의 응답에서 호출해야 합니다.

### 3. 결과 수집

각 validator는 구조화된 블록으로 결과를 반환합니다:

```
---drift-validator-result---
status: success | failed
result_file: .claude/tmp/{session-id}-drift-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
---end-drift-validator-result---
```

```
---export-validator-result---
status: success | failed
result_file: .claude/tmp/{session-id}-export-{dir-safe-name}.md
directory: {directory}
export_coverage: {0-100}
---end-export-validator-result---
```

### 4. 통합 보고서 생성

결과 파일들을 Read하여 다음 형식으로 통합 보고서 생성:

```markdown
# CLAUDE.md 검증 보고서

## Completeness (CLAUDE.md 완성도)

| 상태 | 개수 |
|------|------|
| Complete | 5 |
| Missing | 2 |
| Unexpected | 1 |

**Missing (생성 필요):**
- src/utils (소스 3개)
- src/api (소스 5개)

**Unexpected (삭제 검토):**
- docs/examples (소스 0개, 하위 1개)

## 요약

| 디렉토리 | Drift 이슈 | Export 커버리지 점수 | 상태 |
|----------|-----------|------------|------|
| src/auth | 0 | 95% | 양호 |
| src/utils | 2 | 72% | 개선 권장 |

## 상세 결과

### src/auth
#### Drift 검증
(drift-validator 결과 파일 내용)

#### Export 커버리지 검증
(export-validator 결과 파일 내용)

### src/utils
...
```

### 5. 임시 파일 정리

`.claude/tmp/{session-id}-{prefix}-{target}` 형태의 임시 파일들은 세션 종료 시 자동으로 정리됩니다.

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | Drift 이슈 0개 AND Export 커버리지 점수 90% 이상 |
| **개선 권장** | Drift 1-2개 OR Export 커버리지 점수 70-89% |
| **개선 필요** | Drift 3개 이상 OR Export 커버리지 점수 70% 미만 |

## 출력 예시

```
/validate src/

CLAUDE.md 검증 보고서
=====================

Completeness (CLAUDE.md 완성도)
-------------------------------
Complete: 3개 | Missing: 1개 | Unexpected: 0개

⚠ Missing (생성 필요):
  - src/api (소스 5개)

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 권장: 1개
- 개선 필요: 1개

상세 결과
---------

src/auth (양호)
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (개선 권장)
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  Export 커버리지: 78% (14/18 예측 성공)

src/legacy (개선 필요)
  Drift: 5개 이슈
    - UNCOVERED: 3개 파일이 Structure에 없음
    - MISMATCH: 2개 시그니처 불일치
  Export 커버리지: 45% (9/20 예측 성공)
```

## 관련 컴포넌트

- `core/target/release/claude-md-core audit`: CLAUDE.md 완성도 검증 (CLI)
- `agents/drift-validator.md`: 코드-문서 일치 검증
- `agents/export-validator.md`: Export 커버리지 검증
