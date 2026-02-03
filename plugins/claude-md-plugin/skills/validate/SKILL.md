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

### 1. 대상 수집

Glob으로 대상 경로의 모든 CLAUDE.md 수집:

```
Glob("**/CLAUDE.md", path={path})
```

### 2. 결과 디렉토리 생성

```bash
mkdir -p .claude/validate-results
```

### 3. 병렬 검증 실행

각 CLAUDE.md에 대해 두 validator를 **단일 메시지에서 병렬로 Task 호출**:

```
# 반드시 단일 메시지에서 모든 Task를 호출하여 병렬 실행
For each claude_md_file:
  directory = dirname(claude_md_file)

  Task(drift-validator, prompt="검증 대상: {directory}")
  Task(reproducibility-validator, prompt="검증 대상: {directory}")
```

**중요**: 성능 최적화를 위해 모든 Task를 하나의 응답에서 호출해야 합니다.

### 4. 결과 수집

각 validator는 구조화된 블록으로 결과를 반환합니다:

```
---drift-validator-result---
status: success | failed
result_file: .claude/validate-results/drift-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
---end-drift-validator-result---
```

```
---reproducibility-validator-result---
status: success | failed
result_file: .claude/validate-results/repro-{dir-safe-name}.md
directory: {directory}
understanding_score: {0-100}
---end-reproducibility-validator-result---
```

### 5. 통합 보고서 생성

결과 파일들을 Read하여 다음 형식으로 통합 보고서 생성:

```markdown
# CLAUDE.md 검증 보고서

## 요약

| 디렉토리 | Drift 이슈 | 재현성 점수 | 상태 |
|----------|-----------|------------|------|
| src/auth | 0 | 95% | 양호 |
| src/utils | 2 | 72% | 개선 권장 |

## 상세 결과

### src/auth
#### Drift 검증
(drift-validator 결과 파일 내용)

#### 재현성 검증
(reproducibility-validator 결과 파일 내용)

### src/utils
...
```

### 6. 임시 파일 정리

```bash
rm -rf .claude/validate-results/
```

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | Drift 이슈 0개 AND 재현성 점수 90% 이상 |
| **개선 권장** | Drift 1-2개 OR 재현성 점수 70-89% |
| **개선 필요** | Drift 3개 이상 OR 재현성 점수 70% 미만 |

## 출력 예시

```
/validate src/

CLAUDE.md 검증 보고서
=====================

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
  재현성: 95% (18/19 예측 성공)

src/utils (개선 권장)
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  재현성: 78% (14/18 예측 성공)

src/legacy (개선 필요)
  Drift: 5개 이슈
    - UNCOVERED: 3개 파일이 Structure에 없음
    - MISMATCH: 2개 시그니처 불일치
  재현성: 45% (9/20 예측 성공)
```

## 관련 컴포넌트

- `agents/drift-validator.md`: 코드-문서 일치 검증
- `agents/reproducibility-validator.md`: 재현성 검증
