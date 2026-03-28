# Impact Templates

## 영향 수준 분류 규칙

| 섹션 | 비교 단위 | 영향 수준 |
|------|----------|----------|
| **Purpose** | 텍스트 변경 | HIGH (모듈 역할 변경) |
| **Requirements** 추가 | 항목 단위 | HIGH (새 제약 → 의존 모듈 코드 영향 가능) |
| **Requirements** 제거 | 항목 단위 | MEDIUM (제약 완화) |
| **Requirements** 수정 | 항목 단위 | HIGH (제약 변경 → 의존 모듈 동작 변경 가능) |
| **Domain Context** | 항목 변경 | LOW (맥락 정보) |

## 보고서 템플릿

```markdown
# 변경 영향 분석: {path}

## 변경 요약

| 섹션 | 변경 유형 | 영향 수준 |
|------|----------|----------|
| Purpose | 변경 | HIGH |
| Requirements | 추가 (2), 수정 (1) | HIGH |
| Domain Context | 수정 (1) | LOW |

## Requirements 변경 상세

### 추가
- `+ 동시 접속 최대 100명`
- `+ UTF-8 인코딩 필수`

### 수정
- `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

## 영향받는 모듈

### HIGH (코드 수정 필요)

#### src/api
- **참조 방식**: CLAUDE.md에서 {path} 참조
- **영향**: Requirements 변경으로 인한 동작 변경 가능
- **추천**: `/validate src/api` → `/compile --path src/api --conflict overwrite`

### LOW (확인만 필요)

(없음)

## 추천 액션

1. `/validate` — 영향받는 모듈 검증
2. `/compile --path src/api --conflict overwrite` — 재컴파일
```
