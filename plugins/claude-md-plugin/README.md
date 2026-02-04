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

### /decompile - 소스코드에서 CLAUDE.md 추출

```
/decompile
```

현재 프로젝트의 모든 디렉토리를 분석하여 CLAUDE.md를 생성합니다. (소스코드 → CLAUDE.md = 바이너리 → 소스 = decompile)

**워크플로우:**
1. 프로젝트 트리 파싱 → CLAUDE.md가 필요한 디렉토리 식별
2. Leaf-first 순서로 각 디렉토리 처리
3. 바운더리 분석 → 코드 분석 → CLAUDE.md 초안 생성 → 스키마 검증

### /compile - CLAUDE.md에서 소스코드 생성

```bash
# 기본 사용 (프로젝트 전체)
/compile

# 특정 경로만 처리
/compile --path src/auth

# 충돌 시 덮어쓰기
/compile --conflict overwrite
```

CLAUDE.md 파일을 기반으로 소스 코드를 컴파일합니다. (CLAUDE.md → 소스코드 = 소스 → 바이너리 = compile) 내부적으로 TDD 워크플로우를 자동 수행합니다.

**옵션:**
| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리할 디렉토리 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 방식 (`skip` \| `overwrite`) |

**내부 TDD 워크플로우:**
1. CLAUDE.md 파싱 → 구조화된 스펙 추출
2. [RED] behaviors 섹션 → 테스트 코드 생성
3. [GREEN] exports + contracts → 구현 코드 생성
4. 테스트 실행 (실패 시 최대 3회 재시도)

### /validate - CLAUDE.md 검증

```bash
# 기본 사용 (프로젝트 전체)
/validate

# 특정 경로만 검증
/validate src/
```

CLAUDE.md 문서의 품질과 코드 일치 여부를 검증합니다.

**검증 항목:**
- **Drift 검증**: Structure, Exports, Dependencies, Behavior가 실제 코드와 일치하는지
- **재현성 검증**: CLAUDE.md만으로 코드를 재현할 수 있는지 (80% 이상 권장)

**결과 상태:**
| 상태 | 조건 |
|------|------|
| **양호** | Drift 이슈 0개 AND 재현성 점수 90% 이상 |
| **개선 권장** | Drift 1-2개 OR 재현성 점수 70-89% |
| **개선 필요** | Drift 3개 이상 OR 재현성 점수 70% 미만 |

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

## 언어 지원

**프로젝트에서 사용하는 언어와 테스트 프레임워크를 자동 감지합니다.**

- 언어 감지: 파일 확장자 기반
- 테스트 프레임워크 감지: 프로젝트 설정 파일 분석

## 향후 계획

- **Round-trip 테스트**: Extract → Generate → Compare

## 라이선스

MIT
