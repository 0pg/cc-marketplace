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
| src/legacy | FAIL(1)| 2 VIOLATED  | 2 위반     | MISSING       | 1 위반   | 개선 필요 |

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
  Drift: 6개 이슈
    - [Requirements VIOLATED] "응답 시간 100ms 이내" — 코드에서 timeout 500ms 설정 발견 (MEDIUM)
    - [Requirements VIOLATED] "UTF-8만 지원" — 코드에서 latin-1 인코딩 사용 발견 (MEDIUM)
    - [Convention MISSING_SUBSECTION] Coding Rules 서브섹션 없음 (HIGH)
    - [Convention CODE_VIOLATION] Naming Rules: 함수명 camelCase 규칙 위반 (snake_case 3건) (MEDIUM)
    - [Boundary PARENT_REFERENCE] ../utils 참조 발견 (line 15) (HIGH)
    - [DEVELOPERS.md MISSING] --strict warning (HIGH)

권장 사항: /resolve로 drift 해소를 진행하세요.
```
