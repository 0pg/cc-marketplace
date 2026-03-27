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

| 디렉토리   | 스키마 | Requirements | Domain Context | Convention | DEVELOPERS.md | Boundary | 상태      |
|------------|--------|-------------|----------------|------------|---------------|----------|-----------|
| src/auth   | PASS   | 0           | 0              | 0          | OK            | 0        | 양호      |
| src/utils  | PASS   | 1 VIOLATED  | 1 STALE        | 0          | OK            | 0        | 개선 필요 |
| src/legacy | FAIL(1)| 2 VIOLATED  | 0              | 1 위반     | MISSING       | 1 위반   | 개선 필요 |

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈 (5 카테고리 모두 정상)

src/utils (개선 필요)
  스키마: PASS
  Drift: 2개 이슈
    - [Requirements VIOLATED] "동시 세션 최대 5개" — 코드에서 MAX_SESSIONS = 10 발견
    - [Domain Context STALE] "Redis 캐시 사용" — 코드에서 Redis 관련 패턴 미발견

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Requirements → fix-schema로 수정 ✓
  Drift: 4개 이슈
    - [Requirements VIOLATED] "응답 시간 100ms 이내" — 코드에서 timeout 500ms 설정 발견
    - [Requirements VIOLATED] "UTF-8만 지원" — 코드에서 latin-1 인코딩 사용 발견
    - [Convention 위반] Naming Rules: 함수명 camelCase 규칙 위반 (snake_case 3건)
    - [Boundary 위반] 형제 모듈 src/auth/CLAUDE.md 직접 참조
  DEVELOPERS.md: MISSING (--strict warning)

권장 사항: /resolve로 drift 해소를 진행하세요.
```
