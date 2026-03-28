# Validate Report Example

```
/validate src/

CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 필요: 2개

| 디렉토리   | 스키마 | Requirements | Convention | DEVELOPERS.md | Boundary | 상태      |
|------------|--------|-------------|------------|---------------|----------|-----------|
| src/auth   | PASS   | 0           | 0          | OK            | 0        | 양호      |
| src/utils  | PASS   | 1 VIOLATED  | 0          | OK            | 0        | 개선 필요 |
| src/legacy | FAIL(1)| 2 VIOLATED  | 1 위반     | MISSING(WARN) | 1 위반   | 개선 필요 |

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈

src/utils (개선 필요)
  스키마: PASS
  Drift: 1개 이슈
    - [Requirements VIOLATED] "동시 세션 최대 5개" — 코드에서 MAX_SESSIONS = 10 발견 (MEDIUM)

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Requirements → fix-schema로 수정 ✓
  Drift: 5개 이슈
    - [Requirements VIOLATED] "응답 시간 100ms 이내" — 코드에서 timeout 500ms 설정 발견 (MEDIUM)
    - [Requirements VIOLATED] "UTF-8만 지원" — 코드에서 latin-1 인코딩 사용 발견 (MEDIUM)
    - [Convention MISSING_SUBSECTION] Coding Rules 서브섹션 없음 (HIGH)
    - [Convention CODE_VIOLATION] 의존성 방향 위반: service → controller import (MEDIUM)
    - [Boundary PARENT_REFERENCE] ../utils 참조 발견 (line 15) (HIGH)
  DEVELOPERS.md: MISSING (WARNING)

Auto-fix 결과
--------------

[1/4] src/utils: Requirements VIOLATED — "동시 세션 최대 5개" vs MAX_SESSIONS = 10
  해소: [Fix Code / Fix Doc / Skip] → Fix Code (/compile 실행)

[2/4] src/legacy: Requirements VIOLATED — "응답 시간 100ms 이내" vs timeout 500ms
  해소: [Fix Code / Fix Doc / Skip] → Fix Doc (Requirement 업데이트)

[3/4] src/legacy: Requirements VIOLATED — "UTF-8만 지원"
  해소: [Fix Code / Fix Doc / Skip] → Skip

[4/4] DEVELOPERS.md가 없는 1개 모듈(src/legacy)에 대해 생성하시겠습니까?
  [생성 / 건너뛰기] → 건너뛰기

재검증을 실행하시겠습니까? [실행 / 건너뛰기] → 건너뛰기

최종 요약: 총 이슈 6개, Auto-fix 2/6개 해소
```
