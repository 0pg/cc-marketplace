# Drift Types Reference

validator agent가 감지하는 4가지 Drift 유형에 대한 상세 설명입니다.

## 1. Structure Drift

CLAUDE.md의 Structure 섹션과 실제 디렉토리 내 파일/디렉토리의 불일치.

| 유형 | 설명 | 원인 |
|------|------|------|
| **UNCOVERED** | 실제 파일이 Structure에 없음 | 새 파일 추가 후 CLAUDE.md 미갱신 |
| **ORPHAN** | Structure에 있으나 실제로 없음 | 파일 삭제 후 CLAUDE.md 미갱신 |

## 2. Exports Drift

CLAUDE.md의 Exports 섹션과 실제 코드의 public export 불일치.

| 유형 | 설명 | 원인 |
|------|------|------|
| **STALE** | 문서의 export가 코드에 없음 | 함수 삭제/이름 변경 후 CLAUDE.md 미갱신 |
| **MISSING** | 코드의 public export가 문서에 없음 | 새 함수 추가 후 CLAUDE.md 미갱신 |
| **MISMATCH** | 시그니처 불일치 | 파라미터/반환 타입 변경 후 CLAUDE.md 미갱신 |

### Export 커버리지 계산

커버리지 = (문서화된 전체 export 수 - STALE 수) ÷ (문서화된 전체 export 수 + MISSING 수) × 100

- `total_exports`: 문서화된 전체 export 수
- `stale_count`: STALE로 판정된 수
- `missing_count`: MISSING으로 판정된 수
- `total_exports + missing_count`가 0이면 coverage = 100

## 3. Dependencies Drift

CLAUDE.md의 Dependencies 섹션과 실제 의존성의 불일치.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **STALE** | 문서의 의존성이 실제로 없음 | internal → 파일 존재 확인, external → 패키지 매니저 확인 |
| **ORPHAN** | 코드에서 사용하지만 문서에 없음 | import문 분석 vs 문서 비교 |

## 4. Behavior Drift

CLAUDE.md의 Behavior 섹션과 실제 동작의 불일치.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **MISMATCH** | 문서화된 시나리오와 실제 동작 불일치 | 테스트 케이스 매칭, 코드 분기문 분석 |
