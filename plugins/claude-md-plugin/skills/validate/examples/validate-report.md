# Validate Report Example

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

| 디렉토리   | 스키마 | Drift 이슈 | Export 커버리지 | 상태      |
|------------|--------|-----------|---------------|-----------|
| src/auth   | PASS   | 0         | 95%           | 양호      |
| src/utils  | PASS   | 2         | 78%           | 개선 권장 |
| src/legacy | FAIL(1)| 5         | 45%           | 개선 필요 |

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (개선 권장)
  스키마: PASS
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  Export 커버리지: 78% (14/18 예측 성공)

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior
  Drift: 5개 이슈
    - UNCOVERED: 3개 파일이 Structure에 없음
    - MISMATCH: 2개 시그니처 불일치
  Export 커버리지: 45% (9/20 예측 성공)
```
