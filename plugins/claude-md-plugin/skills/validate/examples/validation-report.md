# 검증 결과: src/auth

## 요약

- 전체 이슈: 3개
- Structure: 1개
- Exports: 1개
- Dependencies: 0개
- Behavior: 1개

## 스키마 검증

- 결과: PASS

## Export 커버리지

- 커버리지: 95%
- 전체: 19개, 발견: 18개, 누락(STALE): 1개

| 점수 범위 | 해석 |
|----------|------|
| 90-100% | 우수 - CLAUDE.md exports가 코드와 일치 |
| 70-89% | 양호 - 일부 export 보완 필요 |
| 50-69% | 보통 - 주요 export 누락 |
| 0-49% | 미흡 - CLAUDE.md 재작성 권장 |

## IMPLEMENTS.md Presence

- 상태: EXISTS

## 상세

### Structure Drift

#### UNCOVERED (문서에 없는 파일)
- `helper.ts`: 디렉토리에 존재하나 Structure에 없음

#### ORPHAN (실제 없는 파일)
(없음)

### Exports Drift

#### STALE (코드에 없는 export)
- `formatDate(date: Date): string`: 문서에 있으나 코드에 없음

#### MISSING (문서에 없는 export)
(없음)

#### MISMATCH (시그니처 불일치)
(없음)

### Dependencies Drift

(이슈 없음)

### Behavior Drift

#### MISMATCH (동작 불일치)
- "빈 입력 시 빈 배열 반환": 실제로는 null 반환
