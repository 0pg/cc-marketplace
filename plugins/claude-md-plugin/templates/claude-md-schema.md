<!--
  이 파일은 예시와 설명을 위한 문서입니다.
  규칙의 Single Source of Truth: references/shared/schema-rules.yaml
-->

# CLAUDE.md Schema Template (v3.1.0)

이 템플릿은 CLAUDE.md 파일의 표준 구조를 정의합니다.

**CLAUDE.md = 사전학습 인덱스 + 인간 지식 저장소**
- 소스코드가 유일한 Source of Truth
- CLAUDE.md는 AI가 모듈을 빠르게 이해하기 위한 compact index
- DEVELOPERS.md는 "왜(WHY)" 그렇게 결정했는지 맥락을 제공합니다

## 3-문서 체계

```
┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                         │
│                                                             │
│   CLAUDE.md (auto-loaded, compact)                         │
│     → Purpose, Constraints, Domain Context                 │
│                                                             │
│   DEVELOPERS.md (on-demand, detailed)                      │
│     → Domain Context, Invariants, Decision Log,            │
│       Operations, File Map                                 │
│                                                             │
│   .claude/index.md (auto-generated, gitignored)            │
│     → Exports, Behavior, Dependencies, Structure           │
└─────────────────────────────────────────────────────────────┘
```

| 문서 | 역할 | 로드 방식 |
|------|------|----------|
| **CLAUDE.md** | 사전학습 인덱스 | auto-loaded |
| **DEVELOPERS.md** | 인간 지식 저장소 | on-demand |
| **.claude/index.md** | 코드 분석 결과 | auto-generated |

## 필수 섹션 요약 (3 always-required + 2 conditional)

| 섹션 | 필수 | 조건 | "None" 허용 | 설명 |
|------|------|------|-------------|------|
| Purpose | always | — | ✗ | 모듈의 책임을 1-2문장으로 |
| Constraints | always | — | ✓ | 코드가 지켜야 할 규칙 |
| Domain Context | always | — | ✓ | 핵심 맥락 요약 (2-3문장) |
| Instructions | conditional | is_project_root | ✗ | AI 행동 지시 (project root에만) |
| Conventions | conditional | is_project_or_module_root | — | 프로젝트/코드 수준 통합 규칙 |

> 규칙 상세: `references/shared/schema-rules.yaml` 참조

---

## 상세 설명

### 1. Purpose (필수, None 불가)
모듈의 책임을 1-2문장으로 명시합니다.

```markdown
## Purpose
이 모듈은 사용자 인증을 담당합니다.
```

### 2. Constraints (필수, None 허용)
코드가 지켜야 할 규칙입니다. 자기완결적으로 작성합니다 (상위 제약도 필요시 반복).

규칙이 없는 경우 `None`을 명시합니다.

```markdown
## Constraints
None
```

규칙이 있는 경우:

```markdown
## Constraints
- 토큰 만료 시간은 최대 7일 (PCI-DSS)
- refresh token은 secure storage에만 저장
- 동시 세션은 최대 5개
```

### 3. Domain Context (필수, None 허용)
핵심 맥락을 2-3문장으로 요약합니다.

도메인 맥락이 없는 경우 `None`을 명시합니다.

```markdown
## Domain Context
None
```

도메인 맥락이 있는 경우:

```markdown
## Domain Context
JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책을 적용합니다.
Redis 캐시를 사용하여 인증 지연을 최소화합니다.
```

### 4. Instructions (조건부 - project root에만)
AI 행동 지시를 명시합니다. project root CLAUDE.md에만 작성합니다.

```markdown
## Instructions
Always use TypeScript strict mode.
Follow the team's code review process.
```

### 5. Conventions (조건부 - project_root 또는 module_root)

프로젝트/코드 수준 통합 규칙입니다. project_root CLAUDE.md에 필수이며, module_root에서는 optional override로 사용됩니다.

```markdown
## Conventions

### Project Structure
src/ 하위에 기능별 디렉토리 구성.

### Module Boundaries
각 모듈은 자체 CLAUDE.md를 가지며, 순환 의존 금지.

### Naming Conventions
디렉토리: kebab-case, 파일: camelCase

### Language & Runtime
TypeScript 5.0, Node.js 20 LTS

### Coding Rules
- 비동기: async/await 사용, raw Promise 금지
- 타입: strict mode, any 금지

### Naming Rules
- 변수/함수: camelCase
- 클래스/타입: PascalCase
```

**필수 서브섹션 (6개):**

| 서브섹션 | 필수 | 설명 |
|----------|------|------|
| Project Structure | Yes | 디렉토리 구조 규칙 |
| Module Boundaries | Yes | 모듈 책임 규칙, 의존성 방향 |
| Naming Conventions | Yes | 모듈/디렉토리/패키지 네이밍 |
| Language & Runtime | Yes | 주요 언어, 버전, 런타임 |
| Coding Rules | Yes | 기본 코딩 규칙 |
| Naming Rules | Yes | 코드 수준 네이밍 규칙 |

## 참조 규칙

### 허용
- 부모 → 자식: 자식 디렉토리 참조 가능

### 금지
- 자식 → 부모: 부모 디렉토리 참조 불가
- 형제 ↔ 형제: 형제 디렉토리 상호 참조 불가

## 관련 문서

- **DEVELOPERS.md**: WHY(결정근거, 운영맥락)를 정의하는 쌍 문서
- 템플릿: `templates/developers-md-schema.md`

### 불변식

**INV-3: CLAUDE.md ↔ DEVELOPERS.md 쌍 (활성)**
```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
--strict 모드에서 DEVELOPERS.md 부재를 경고로 보고
```

**INV-5: Convention 섹션 배치 규칙**
```
project_root/CLAUDE.md MUST contain ## Conventions
module_root/CLAUDE.md MAY contain ## Conventions (override; 없으면 project_root에서 상속)
싱글 모듈: project_root == module_root → 같은 CLAUDE.md에 배치
```
