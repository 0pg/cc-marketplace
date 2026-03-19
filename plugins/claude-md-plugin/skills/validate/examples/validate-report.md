# Validate Report Example

```
/validate src/

CLAUDE.md 계약 검증 보고서
========================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 위반 발견: 2개

| 디렉토리   | 스키마 | 위반 수 (확인/오탐) | 심각도             | Export 커버리지 | 상태      |
|------------|--------|---------------------|--------------------|---------------|-----------|
| src/auth   | PASS   | 0 (0/0)             | -                  | 95%           | 양호      |
| src/utils  | PASS   | 3 (2/1)             | HIGH:1 MED:1       | 78%           | 위반 발견 |
| src/legacy | FAIL(1)| 7 (5/2)             | CRIT:1 HIGH:2 MED:2| 45%           | 위반 발견 |

추천 액션
---------
- src/utils: `/compile --path src/utils --conflict overwrite`
- src/legacy: 위반 보고서 검토 후 결정
  - CRITICAL 위반 있음 — 시그니처 불일치 수동 확인 필요

상세 결과
---------

src/auth (양호)
  스키마: PASS
  위반: 0개
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (위반 발견)
  스키마: PASS
  위반: 2개 확인 (오탐 1개 제외)
    - HIGH Exports STALE: formatDate → 계약에 있으나 코드에 없음 → /compile 재실행
    - MEDIUM Structure UNCOVERED: helper.ts → 계약 Structure 업데이트 필요
  Export 커버리지: 78%

src/legacy (위반 발견)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior → fix-schema로 수정 완료
  위반: 5개 확인 (오탐 2개 제외)
    - CRITICAL Exports MISMATCH: validateToken 시그니처 불일치
      - 영향: src/api, src/middleware
    - HIGH Exports STALE: 2개
    - MEDIUM Structure UNCOVERED: 1개
    - MEDIUM Convention CODE_VIOLATION: 1개
  Export 커버리지: 45%
```
