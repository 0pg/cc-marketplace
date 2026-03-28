# 프로젝트 건강도: GOOD

## 요약

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 5 | - |
| 스키마 유효 | 4/5 (80%) | WARNING |
| Compile 신선도 | 3/5 FRESH | WARNING |
| Convention | PASS (6/6 서브섹션) | OK |
| DEVELOPERS.md 쌍 | 4/5 (80%) | WARNING |

## 모듈별 상태

| 모듈 | 스키마 | Compile | DEVELOPERS.md |
|------|--------|---------|---------------|
| src/auth | PASS | FRESH | EXISTS |
| src/api | PASS | STALE | EXISTS |
| src/utils | PASS | FRESH | EXISTS |
| src/legacy | FAIL (1) | STALE | MISSING |
| src/new | PASS | UNCOMPILED | EXISTS |

## 추천 액션

1. `src/legacy`: 스키마 오류 수정 → `/validate src/legacy`
2. `src/api`, `src/legacy`: 재컴파일 필요 → `/compile`
3. `src/new`: 첫 컴파일 필요 → `/compile --path src/new`
4. `src/legacy`: DEVELOPERS.md 생성 필요 → `/decompile src/legacy`
