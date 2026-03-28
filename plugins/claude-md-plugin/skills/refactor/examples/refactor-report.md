# Refactor 결과: src/auth (split)

## 분할 전

```
src/auth/
├── CLAUDE.md          (Purpose: JWT 기반 인증 및 세션 관리)
├── DEVELOPERS.md
├── auth.ts
├── token.ts
└── session.ts
```

Requirements: 4개
- 토큰 만료 최대 7일
- UTF-8 인코딩 필수
- 동시 세션 최대 5개
- 비활성 세션 30분 만료

## 분할 계획

Requirements 그루핑:
- Group A (토큰): 토큰 만료 최대 7일, UTF-8 인코딩 필수
- Group B (세션): 동시 세션 최대 5개, 비활성 세션 30분 만료

## 영향 분석

| 모듈 | 영향 수준 | 설명 |
|------|----------|------|
| src/api | HIGH | src/auth 참조 → 경로 변경 필요 |
| src/middleware | HIGH | src/auth 참조 → 경로 변경 필요 |

## 분할 후

```
src/auth/
├── CLAUDE.md          (Purpose: 인증 모듈 루트)
├── token/
│   ├── CLAUDE.md      (Purpose: JWT 토큰 인증)
│   ├── DEVELOPERS.md
│   └── token.ts
└── session/
    ├── CLAUDE.md      (Purpose: 세션 관리)
    ├── DEVELOPERS.md
    └── session.ts
```

## 생성/수정된 파일

- Created: src/auth/token/CLAUDE.md + DEVELOPERS.md
- Created: src/auth/session/CLAUDE.md + DEVELOPERS.md
- Updated: src/auth/CLAUDE.md (Purpose 축소, Requirements 이동)

## 다음 단계

1. src/api/CLAUDE.md 참조 경로 업데이트: src/auth → src/auth/token
2. src/middleware/CLAUDE.md 참조 경로 업데이트: src/auth → src/auth/token
3. `/compile --all --conflict overwrite` — 전체 재컴파일
4. `/validate` — 검증
