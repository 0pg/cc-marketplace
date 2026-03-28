# Diff-Spec Templates

## 섹션별 비교 분류 규칙

### Purpose 비교
- 텍스트 변경 여부: CHANGED / UNCHANGED

### Requirements 비교 (항목 단위)

각 requirement를 매칭하여 변경 분류:

| 변경 유형 | 조건 |
|-----------|------|
| **ADDED** | 현재에만 존재 |
| **REMOVED** | 이전에만 존재 |
| **MODIFIED** | 양쪽에 유사 항목 존재, 내용 다름 |
| **UNCHANGED** | 양쪽 동일 |

### Domain Context 비교

항목별 변경 분류 (ADDED / REMOVED / MODIFIED / UNCHANGED)

### Conventions 비교 (서브섹션 단위)

각 서브섹션별 변경 여부:
- Project Structure, Module Boundaries, Naming Conventions
- Language & Runtime, Coding Rules, Naming Rules

## 보고서 템플릿

```markdown
# 시맨틱 Diff: {path}

**비교:** {ref} → 현재 (working copy)

## 요약

| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Purpose | - | - | 1 | CHANGED |
| Requirements | 2 | 1 | 1 | BREAKING |
| Domain Context | 1 | 0 | 0 | MODIFIED |
| Conventions | 0 | 0 | 1 | MODIFIED |

## Purpose 변경

- 이전: "JWT 기반 인증 모듈"
+ 현재: "JWT 및 OAuth2 기반 인증 모듈"

## Requirements 변경

### ADDED
- `+ 동시 세션 최대 5개`
- `+ OAuth2 PKCE 필수`

### REMOVED
- `- 레거시 MD5 해시 지원` [BREAKING]

### MODIFIED
- `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

### UNCHANGED
- `UTF-8 인코딩만 허용`

## Domain Context 변경

### ADDED
- `+ OAuth2 IdP: Google, GitHub 지원`

## Conventions 변경

### MODIFIED
- `Coding Rules`: 린트 규칙 추가
```
