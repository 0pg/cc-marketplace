<!--
  이 파일은 예시와 설명을 위한 문서입니다.
  규칙의 Single Source of Truth: core/schema-rules.yaml
-->

# CLAUDE.md Schema Template (v4.0.0)

이 템플릿은 CLAUDE.md 파일의 표준 구조를 정의합니다.

**CLAUDE.md = Primary SSOT — PM의 요구사항 문서**
- CLAUDE.md는 PM이 읽고 쓸 수 있는 비즈니스 요구사항 문서
- DEVELOPERS.md는 개발자가 Requirements를 시스템 레벨로 구체화한 Derived Spec
- 소스코드는 문서에서 파생된 Derived Artifact

## 2-문서 체계

```
┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                         │
│                                                             │
│   CLAUDE.md (Primary SSOT, auto-loaded)                    │
│     → Purpose, Requirements, Domain Context                │
│                                                             │
│   DEVELOPERS.md (Derived Spec, on-demand)                  │
│     → Constraints, Technical Context, Decision Log,        │
│       Operations                                           │
└─────────────────────────────────────────────────────────────┘
```

| 문서 | 역할 | 로드 방식 |
|------|------|----------|
| **CLAUDE.md** | Primary SSOT (PM 요구사항) | auto-loaded |
| **DEVELOPERS.md** | Derived Spec (개발자 명세) | on-demand |

## 필수 섹션 요약 (3 always-required + 2 conditional)

| 섹션 | 필수 | 조건 | "None" 허용 | 설명 |
|------|------|------|-------------|------|
| Purpose | always | — | ✗ | 모듈의 존재 이유 (비즈니스 가치) |
| Requirements | always | — | ✓ | 비즈니스 요구사항 (사용자 관점, 검증 가능한 문장) |
| Domain Context | always | — | ✓ | 비즈니스 제약 배경 (규정, 레거시, 조직적 이유) |
| Instructions | conditional | is_project_root | ✗ | AI 행동 지시 (project root에만) |
| Conventions | conditional | is_project_or_module_root | — | 프로젝트/코드 수준 통합 규칙 |

> 규칙 상세: `core/schema-rules.yaml` 참조

---

## 상세 설명

### 1. Purpose (필수, None 불가)
모듈의 존재 이유를 비즈니스 가치 중심으로 1-2문장으로 명시합니다.

```markdown
## Purpose
사용자 인증을 담당. 보안 규정 준수와 원활한 사용자 경험 제공.
```

### 2. Requirements (필수, None 허용)
비즈니스 요구사항을 사용자 관점으로 기술합니다. PM이 읽고 쓸 수 있어야 합니다.

요구사항이 없는 경우 `None`을 명시합니다.

```markdown
## Requirements
None
```

요구사항이 있는 경우:

```markdown
## Requirements
- 만료된 토큰으로 접근 시 자동 갱신, 사용자 재로그인 불필요
- 동시 로그인 기기 최대 5개, 초과 시 가장 오래된 세션 종료
- PCI-DSS 규정에 따른 토큰 수명 제한
```

**Requirements 작성 원칙:**
- 사용자 관점의 동작 기술
- 기술 용어 최소화
- 비즈니스 가치 중심
- 모호함 허용 (구체화는 DEVELOPERS.md Constraints에서)

### 3. Domain Context (필수, None 허용)
비즈니스 제약 배경을 2-3문장으로 요약합니다.

도메인 맥락이 없는 경우 `None`을 명시합니다.

```markdown
## Domain Context
None
```

도메인 맥락이 있는 경우:

```markdown
## Domain Context
- PCI-DSS 컴플라이언스에 따라 토큰 만료 기간 제한
- 레거시 시스템과의 호환성을 위해 UUID v1 형식 지속 지원
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

- **DEVELOPERS.md**: Derived Spec — CLAUDE.md Requirements를 시스템 레벨로 구체화하는 쌍 문서
- 템플릿: `references/shared/developers-md-schema.md`

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
