# claude-md-plugin

> CLAUDE.md를 Source of Truth로 사용하는 문서-코드 동기화 플러그인

## 개요

기존의 "소스코드 → 문서" 접근법을 역전시켜 **CLAUDE.md가 명세**가 되고, **소스코드가 산출물**이 되는 패러다임을 제공합니다.

```
기존: 소스코드 → (분석) → 문서
제안: CLAUDE.md → (생성) → 소스코드
```

## 설치

Claude Code marketplace에서 설치:

```bash
claude marketplace install claude-md-plugin
```

## 사용법

### 소스코드에서 CLAUDE.md 추출

```
/extract
```

현재 프로젝트의 모든 디렉토리를 분석하여 CLAUDE.md를 생성합니다.

## 핵심 개념

### CLAUDE.md 배치 규칙

다음 조건 중 하나라도 충족하는 디렉토리에 CLAUDE.md가 존재해야 합니다:
- 1개 이상의 소스코드 파일이 존재
- 2개 이상의 하위 디렉토리가 존재

### 트리 구조 의존성

```
project/CLAUDE.md
    │
    ├──► src/CLAUDE.md
    │        │
    │        └──► src/auth/CLAUDE.md
    │
    └──► tests/CLAUDE.md
```

- **부모 → 자식**: 참조 가능
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

### CLAUDE.md 스키마

```markdown
# {디렉토리명}

## Purpose
이 디렉토리의 책임 (1-2문장)

## Structure
- subdir/: 설명 (상세는 subdir/CLAUDE.md 참조)
- file.ext: 역할

## Exports
### Functions
- `FunctionName(params) ReturnType`

### Types
- `TypeName { fields }`

## Dependencies
- external: package v1.2.3

## Behavior
- 정상 케이스: input → expected output
- 에러 케이스: invalid input → specific error

## Constraints
- 제약사항
```

## CLI 도구

플러그인에 포함된 Rust CLI 도구:

```bash
# 트리 파싱 - CLAUDE.md가 필요한 디렉토리 식별
claude-md-core parse-tree --root . --output tree.json

# 바운더리 결정 - 디렉토리의 책임 범위 분석
claude-md-core resolve-boundary --path src/auth --output boundary.json

# 스키마 검증 - CLAUDE.md 형식 검증
claude-md-core validate-schema --file CLAUDE.md --output validation.json
```

## 향후 계획

- **Validator Agent**: 생성된 CLAUDE.md로 코드 재현 시 80% 일치율 검증
- **Generator Agent**: CLAUDE.md로부터 소스코드 자동 생성

## 라이선스

MIT
