# CLAUDE.md Plugin PRD

## 프로젝트 의도

현대 소프트웨어 개발에서 문서와 코드의 불일치는 고질적인 문제다. 코드가 변경되면 문서는 뒤처지고, 문서를 업데이트해도 코드와의 괴리는 시간이 지날수록 벌어진다.

이 플러그인은 **패러다임을 역전**시킨다.

```
기존: 소스코드 → (분석) → 문서
제안: CLAUDE.md → (생성) → 소스코드
```

CLAUDE.md가 **Source of Truth**가 되고, 소스코드 파일들은 CLAUDE.md로부터 파생되는 **binary artifact**로 취급한다. 이로써 문서와 코드의 불일치 문제를 구조적으로 해결한다.

---

## 프로젝트 목적

### Primary Goal

**CLAUDE.md만 읽고 Claude Code가 동일한 구조와 기능의 소스코드를 80% 이상의 확률로 재현할 수 있는 문서 체계를 구축한다.**

### Secondary Goals

1. **개발 워크플로우 변경**: 기능 변경 시 코드가 아닌 CLAUDE.md를 먼저 수정
2. **일관된 코드베이스**: 동일한 CLAUDE.md에서 생성된 코드는 일관된 스타일과 구조를 가짐
3. **온보딩 가속화**: 새 팀원이 CLAUDE.md만 읽고 시스템 이해 가능
4. **AI 협업 최적화**: Claude Code가 코드베이스를 이해하는 비용 최소화

---

## 컨셉 및 아키텍처

### 핵심 개념

#### 1. CLAUDE.md = 소스코드의 스펙

CLAUDE.md는 해당 디렉터리(모듈)의 완전한 명세서다. 이 문서만으로:
- 어떤 파일들이 존재해야 하는지
- 각 파일이 어떤 인터페이스를 제공하는지
- 어떤 동작을 해야 하는지

를 알 수 있어야 한다.

#### 2. 소스코드 = Binary Artifact

`.go`, `.rs`, `.kt`, `.ts` 등 실제 소스코드 파일들은 CLAUDE.md로부터 생성되는 산출물이다. 이론적으로 모든 소스코드를 삭제해도 CLAUDE.md만 있으면 재생성 가능해야 한다.

#### 3. 트리 구조 의존성

```
project/CLAUDE.md
    │
    ├──► src/CLAUDE.md
    │        │
    │        ├──► src/api/CLAUDE.md
    │        │
    │        └──► src/domain/CLAUDE.md
    │
    └──► tests/CLAUDE.md
```

- **부모 → 자식**: 참조 가능 (자식 목록, 역할, 연결관계 명시)
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

각 CLAUDE.md는 자신의 바운더리 내에서 **self-contained**여야 한다.

### CLAUDE.md 배치 규칙

다음 조건 중 하나라도 충족하는 디렉터리에 CLAUDE.md가 존재해야 한다:
- 1개 이상의 소스코드 파일이 존재
- 2개 이상의 하위 디렉터리가 존재

### CLAUDE.md 스키마

```markdown
# {디렉터리명}

## Purpose
이 디렉터리의 책임 (1-2문장)

## Structure
- subdir/: 간단한 설명 (상세는 subdir/CLAUDE.md 참조)
- file.ext: 이 파일의 역할

## Exports (시그니처 레벨 - 정확하게)
### Functions
- `FunctionName(param1 Type1, param2 Type2) (ReturnType, error)`

### Types
- `TypeName { Field1 Type1, Field2 Type2 }`

### Constants
- `CONSTANT_NAME`: 설명 및 값

## Dependencies
- external: package/path v1.2.3

## Behavior (시나리오로 - 명확하게)
- 정상 케이스: input → expected output
- 엣지 케이스: condition → expected result
- 에러 케이스: invalid input → specific error

## Constraints
- 지켜야 할 규칙, 제약사항
```

### 상세도 원칙

| 섹션 | 상세도 | 이유 |
|------|--------|------|
| **Exports** | 시그니처 레벨 | 모듈 간 계약, 정확해야 재현 가능 |
| **Internal** | 개념 레벨 | 구현 자유도 허용 |
| **Behavior** | 시나리오 레벨 | 엣지케이스 명확화 |

### 아키텍처

```
┌─────────────────────────────────────────────────────────────┐
│                    CLAUDE.md Plugin                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  Extractor  │    │  Validator  │    │  Generator  │     │
│  │    Agent    │    │    Agent    │    │    Agent    │     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘     │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Core Engine                        │   │
│  │  - Tree Parser: CLAUDE.md 트리 구조 파싱             │   │
│  │  - Boundary Resolver: 각 노드의 바운더리 결정        │   │
│  │  - Schema Validator: CLAUDE.md 스키마 검증           │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   File System                        │   │
│  │  - CLAUDE.md files                                   │   │
│  │  - Source code files (binary)                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Extractor Agent
기존 소스코드를 분석하여 CLAUDE.md 초안을 생성한다.
- Input: 소스코드 디렉터리
- Output: CLAUDE.md 파일들

#### Validator Agent
CLAUDE.md의 품질을 검증한다. 해당 CLAUDE.md만 읽고 소스코드를 재생성했을 때 기존과 동일한 결과가 나오는지 확인한다.
- Input: CLAUDE.md + 현재 소스코드
- Output: Pass/Fail + 불일치 리포트

#### Generator Agent
CLAUDE.md를 읽고 소스코드를 생성한다.
- Input: CLAUDE.md
- Output: 소스코드 파일들

### 검증 프로세스

```
┌─────────────────────────────────────────────────────────────┐
│                    Validation Flow                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   CLAUDE.md ──────┬──────────────────────────────────►      │
│        │          │                                          │
│        │     [Generator Agent]                               │
│        │          │                                          │
│        ▼          ▼                                          │
│   현재 소스코드   생성된 소스코드                             │
│   (ground truth)  (prediction)                               │
│        │          │                                          │
│        └────┬─────┘                                          │
│             │                                                │
│             ▼                                                │
│      [Validator Agent]                                       │
│             │                                                │
│             ▼                                                │
│   ┌─────────────────────┐                                   │
│   │ 1. 구조 검증         │ 파일명/디렉터리명 일치 여부        │
│   │ 2. 인터페이스 검증   │ Export 시그니처 일치 여부          │
│   │ 3. 동작 검증         │ Behavior 시나리오 충족 여부        │
│   └─────────────────────┘                                   │
│             │                                                │
│             ▼                                                │
│      Pass (≥80%) / Fail                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### 검증 순서

1. **Leaf 노드부터 검증**: 의존성이 없는 최하위 디렉터리부터 시작
2. **각 노드는 자기 바운더리만 검증**: 직접 소유한 파일만 검증 대상
3. **하위 디렉터리는 존재 여부만 확인**: 내부 검증은 해당 CLAUDE.md 책임

---

## 제약조건

### 불변식 (Invariants)

#### INV-1: 트리 구조 의존성
```
∀ node ∈ CLAUDE.md tree:
  node.dependencies ⊆ node.children
```
모든 CLAUDE.md는 자신의 자식 노드만 참조할 수 있다. 부모, 형제 참조는 금지된다.

#### INV-2: Self-contained 바운더리
```
∀ node ∈ CLAUDE.md tree:
  validate(node) = validate(node.claude_md, node.direct_files)
```
각 CLAUDE.md의 검증은 해당 CLAUDE.md와 직접 소유한 파일들만으로 완결되어야 한다.

#### INV-3: 재현 가능성
```
∀ node ∈ CLAUDE.md tree:
  P(generate(node.claude_md) ≈ node.current_files) ≥ 0.8
```
각 CLAUDE.md로부터 생성한 코드가 현재 코드와 구조적/기능적으로 동등할 확률이 80% 이상이어야 한다.

#### INV-4: Export 명세 완전성
```
∀ node ∈ CLAUDE.md tree:
  node.exports = complete_public_interface(node.direct_files)
```
CLAUDE.md의 Exports 섹션은 해당 모듈의 public interface를 시그니처 레벨로 완전하게 명세해야 한다.

### 불변조건 (Constraints)

#### CON-1: CLAUDE.md 배치 필수 조건
디렉터리에 CLAUDE.md가 존재해야 하는 조건:
- 해당 디렉터리에 1개 이상의 소스코드 파일이 존재하거나
- 해당 디렉터리에 2개 이상의 하위 디렉터리가 존재

#### CON-2: 스키마 준수
모든 CLAUDE.md는 정의된 스키마(Purpose, Structure, Exports, Dependencies, Behavior, Constraints)를 따라야 한다.

#### CON-3: 상세도 레벨
- Exports: 반드시 시그니처 레벨 (함수명, 파라미터 타입, 리턴 타입)
- Behavior: 반드시 시나리오 레벨 (input → output 형태)
- Internal implementation: 개념 레벨 허용

#### CON-4: 단일 책임
각 CLAUDE.md는 자신의 바운더리(디렉터리) 내의 파일들에 대해서만 책임진다.
- 직접 소유 파일: 구조, 인터페이스, 동작 모두 명세
- 하위 디렉터리: 존재와 역할만 명시, 상세는 해당 디렉터리의 CLAUDE.md가 담당

#### CON-5: 변경 워크플로우
기능 변경 시:
1. 먼저 CLAUDE.md 수정
2. 이후 CLAUDE.md 기반으로 소스코드 변경/생성
3. Validation 실행으로 일관성 확인

---

## 용어 정의

| 용어 | 정의 |
|------|------|
| **CLAUDE.md** | 디렉터리(모듈)의 완전한 명세서. Source of Truth. |
| **Binary** | 소스코드 파일 (`.go`, `.rs`, `.kt` 등). CLAUDE.md로부터 파생되는 산출물. |
| **바운더리** | 각 CLAUDE.md가 책임지는 범위. 해당 디렉터리 내 직접 소유 파일들. |
| **트리 구조** | CLAUDE.md 간의 계층적 의존 관계. 부모→자식 단방향만 허용. |
| **Self-contained** | 외부 참조 없이 자기 완결적인 상태. |
| **재현 가능성** | CLAUDE.md만으로 동일한 구조/기능의 코드를 생성할 수 있는 정도. |
| **Leaf 노드** | 하위 CLAUDE.md가 없는 최하위 디렉터리의 CLAUDE.md. |

---

## 성공 지표

1. **재현율 80% 달성**: 임의의 CLAUDE.md에서 코드 생성 시 구조/기능 일치율
2. **문서-코드 동기화**: CLAUDE.md 변경 없이 소스코드만 변경된 경우 Validation 실패
3. **온보딩 시간 단축**: 새 팀원이 CLAUDE.md만으로 모듈 이해 가능

