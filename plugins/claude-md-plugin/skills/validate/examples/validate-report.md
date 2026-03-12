# Validate Report Example

```
/validate src/

CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 수정 완료: 1개
- 개선 필요: 1개 (일부 수정 실패)

| 디렉토리   | 스키마 | Drift | 확인됨 | 오탐 | 수정됨 | Export 커버리지 | 상태      |
|------------|--------|-------|--------|------|--------|---------------|-----------|
| src/auth   | PASS   | 0     | -      | -    | -      | 95%           | 양호      |
| src/utils  | PASS   | 3     | 2      | 1    | 2      | 78%→90%       | 수정 완료 |
| src/legacy | FAIL(1)| 5     | 4      | 1    | 3      | 45%→60%       | 개선 필요 |

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (수정 완료)
  스키마: PASS
  Drift: 3개 이슈 → 재검증: 2개 확인, 1개 오탐 → 2개 수정 완료
    - STALE: formatDate → 확인됨 → Exports에서 제거 ✓
    - MISSING: parseNumber → 확인됨 → Exports에 추가 ✓
    - MISSING: _internalHelper → 오탐 (private 헬퍼)
  Export 커버리지: 78%→90%

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior → fix-schema로 수정 ✓
  Drift: 5개 이슈 → 재검증: 4개 확인, 1개 오탐 → 3개 수정 완료, 1개 실패
    - UNCOVERED: 3개 파일 → 2개 확인, 1개 오탐 → 2개 Structure에 추가 ✓
    - MISMATCH: 2개 시그니처 → 모두 확인 → 1개 수정 ✓, 1개 수정 실패 ✗
  Export 커버리지: 45%→60%
```
