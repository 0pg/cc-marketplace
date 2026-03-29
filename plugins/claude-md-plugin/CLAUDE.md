# claude-md-plugin

## Purpose

**CLAUDE.md가 Primary SSOT — PM의 요구사항 문서.**

비즈니스 요구사항을 CLAUDE.md로 정의하고, DEVELOPERS.md로 시스템 레벨로 구체화하여,
소스코드를 파생 산출물로 생성하는 문서-코드 동기화 플러그인.

## Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v10                       │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM 요구사항)                     │
│         │                                                    │
│         ├──── /impl ──→     요구사항 → CLAUDE.md 정의        │
│         ├──── /compile ──→  CLAUDE.md 기반 코드 생성         │
│         ├──── /validate ──→ 문서-코드 일치 검증              │
│         └──── /decompile ──→ 소스코드 → CLAUDE.md 추출       │
│                                                              │
│   DEVELOPERS.md (Derived Spec — 개발자 명세)                 │
│         │                                                    │
│         └──── Constraints = 테스트 생성 원천                 │
│                                                              │
│   Source Code (Derived Artifact)                             │
└──────────────────────────────────────────────────────────────┘
```

| 개념 | 역할 | 설명 |
|------|------|------|
| **Primary SSOT** | CLAUDE.md | PM의 요구사항 문서 (Purpose, Requirements, Domain Context) |
| **Derived Spec** | DEVELOPERS.md | 개발자 명세 (Constraints, Technical Context, Decision Log, Operations) |
| **Derived Artifact** | Source Code | CLAUDE.md에서 파생된 코드 |

**불일치 시**: 코드를 재생성한다 (CLAUDE.md가 SSOT).

## 2-Document System

```
module/
├── CLAUDE.md              ← Human-authored / Auto-loaded / 200-600 tok
│   PM의 요구사항 문서. 코드 수정 시 즉시 알아야 할 규칙과 맥락.
│   Claude Code가 계층적으로 자동 로드.
│
└── DEVELOPERS.md          ← Human-authored / On-demand / 선택적
    Derived Spec. Requirements를 시스템 레벨로 구체화.
    /compile이 테스트를 생성하는 원천.
```

### CLAUDE.md Schema (v4.0)

| 섹션 | 존재 규칙 | None 허용 | 설명 |
|------|----------|----------|------|
| `## Purpose` | 항상 필수 | X | 모듈의 존재 이유 (비즈니스 가치) |
| `## Requirements` | 항상 필수 | O | 비즈니스 요구사항 (사용자 관점, 검증 가능한 문장) |
| `## Domain Context` | 항상 필수 | O | 비즈니스 제약 배경 (규정, 레거시, 조직적 이유) |
| `## Conventions` | project/module root 필수 | X | **있음 (override)** — 부모와 다른 부분만 작성 |
| `## Instructions` | **project root only** | X | AI 행동 지시 (project root에서 전역 적용) |

### DEVELOPERS.md Schema

| 섹션 | 필수 | None 허용 | 내용 |
|------|------|----------|------|
| `## Constraints` | O | O | 정밀한 입출력 계약 — 테스트 변환 가능 |
| `## Technical Context` | O | O | 기술 선택과 근거 (라이브러리, 알고리즘, 패턴) |
| `## Decision Log` | X | O | ADR 스타일: 맥락/결정/근거 |
| `## Operations` | X | O | Gotchas, 배포, 모니터링 |

### Conventions Section

`## Conventions`는 project/module root CLAUDE.md에 배치됩니다.

필수 6개 서브섹션:
- `### Project Structure` — 디렉토리 구조 규칙, 레이어링 패턴
- `### Module Boundaries` — 모듈 책임 규칙, 의존성 방향
- `### Naming Conventions` — 모듈/디렉토리/패키지 네이밍
- `### Language & Runtime` — 주요 언어, 버전, 런타임
- `### Coding Rules` — 린터 검증 불가 기본 코딩 규칙
- `### Naming Rules` — 변수/함수/클래스/상수 네이밍

**DRY 원칙**: Claude Code는 CLAUDE.md를 계층적으로 로드하므로, project_root Conventions는
하위 모듈에서 자동 참조됩니다. module_root에는 project_root와 다른 내용만 작성합니다.

### 트리 구조 의존성
- **부모 → 자식**: 참조 가능
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

## Architecture

### Session File Pattern

v10의 핵심 인터페이스: SKILL이 문서에서 정보를 추출하여 세션 파일을 생성하고, Agent가 세션 파일을 소비.

```
SKILL (Entry Point)
  │
  ├── CLI 호출 (결정론적 검증/분석)
  ├── CLAUDE.md + DEVELOPERS.md 읽기
  ├── 세션 파일 Write (${TMP_DIR}{type}-session-{dir-safe}.md)
  │
  └── Task(Agent)
        │
        ├── Skill("superpowers:{component}") 로드
        ├── 세션 파일 Read (사전 추출된 스펙)
        ├── 비즈니스 로직 실행
        └── 결과 파일 저장 + result block 반환
```

### Active Workflows (Core 4)

#### /impl (요구사항 → CLAUDE.md)

```
User: /impl "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl SKILL                                  │
│                                             │
│ 1. Bash(scan-claude-md) → 인덱스 생성       │
│ 2. decompose 세션 파일 생성                 │
│ 3. Task(decompose agent) → 분해 계획        │
│ 4. scope 분기:                             │
│    single → Task(impl agent) 1개           │
│    multi  → 승인 → Task(impl agent) × N    │
│             root-first, 병렬 최대 3         │
│ 5. git diff 표시                            │
└─────────────────────────────────────────────┘
        │
        ├─ scope=single ──────────────────────┐
        │                                     ▼
        │                    ┌─────────────────────────────────────┐
        │                    │ decompose AGENT                     │
        │                    │                                     │
        │                    │ 1. Scope Classification             │
        │                    │    single → 조기 종료               │
        │                    │    multi  → Phase 2-4 실행          │
        │                    │ 2. Module Identification             │
        │                    │ 3. Requirement Distribution         │
        │                    │ 4. Tree Validation (INV-1)          │
        │                    │ 5. decompose-result.json 저장       │
        │                    └─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl AGENT (single 모드)                    │
│ ⚡ Skill("superpowers:brainstorming")       │
│                                             │
│ 1. 요구사항 추출 + completeness 평가        │
│ 2. 의존성 탐색 (inline, 인덱스 기반)        │
│ 3. AskUserQuestion → 명확화 (최대 2회)      │
│ 4. CLAUDE.md + DEVELOPERS.md 생성           │
│ 5. validate-schema 검증                     │
│ 6. Plan Preview → 사용자 승인               │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ impl AGENT (parallel 모드, scope=multi)     │
│ (brainstorming 생략)                        │
│                                             │
│ 1. 세션 파일에서 target_path 확인           │
│ 2. CLAUDE.md + DEVELOPERS.md 생성           │
│ 3. validate-schema 검증                     │
│ AskUserQuestion 금지 — best-effort 처리     │
└─────────────────────────────────────────────┘
```

#### /compile (CLAUDE.md → 소스코드)

```
User: /compile [--all] [--conflict skip|overwrite] [--dry-run] [--validate]
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL                               │
│                                             │
│ 1. 대상 결정 (--all 또는 incremental)       │
│ 2. 대상별 세션 파일 생성                    │
│    (Requirements + Constraints + Conventions)│
│ 3. leaf-first 정렬, 병렬 배치 (최대 3)     │
│ 4. Task(compiler) per target               │
│ 5. git diff --stat                         │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ compiler AGENT                              │
│ ⚡ Skill("superpowers:tdd")                 │
│                                             │
│ RED: Constraints → 테스트 생성              │
│ GREEN: 구현, 최대 3회 재시도                │
│ REFACTOR: Conventions 적용 + 회귀 테스트    │
└─────────────────────────────────────────────┘
```

#### /validate (문서-코드 일치 검증)

```
User: /validate [path] [--strict] [--report-only]
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL                              │
│                                             │
│ 1. Glob → CLAUDE.md 수집                    │
│ 2. Deterministic CLI 검증                   │
│    (schema, convention, boundary, INV-3)    │
│ 3. 세션 파일 생성 (문서 내용 + CLI 결과)    │
│ 4. Task(validator) 병렬 배치 (최대 3)      │
│ 5. Auto-fix (Interactive)                  │
│ 6. 통합 보고서                              │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ validator AGENT                             │
│ ⚡ Skill("superpowers:verification")        │
│                                             │
│ 1. Requirements Drift 검출                  │
│ 2. Convention CODE_VIOLATION 검출           │
│ 3. DEVELOPERS.md Content Drift (strict)    │
└─────────────────────────────────────────────┘
```

#### /decompile (소스코드 → CLAUDE.md)

```
User: /decompile [path]
        │
        ▼
┌─────────────────────────────────────────────┐
│ decompile SKILL                             │
│                                             │
│ 1. Bash(parse-tree) → 디렉토리 구조        │
│ 2. leaf-first 정렬                          │
│ 3. 대상별 세션 파일 생성 + Task(decompiler) │
│ 4. git diff --stat                         │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ decompiler AGENT                            │
│ (superpowers 조합 없음 — 추출 작업)         │
│                                             │
│ 1. resolve-boundary + analyze-code          │
│ 2. format-analysis → 요약                  │
│ 3. CLAUDE.md + DEVELOPERS.md 생성           │
│ 4. validate-schema 검증                     │
└─────────────────────────────────────────────┘
```

### 설계 원칙

| 컴포넌트 | 역할 | 오케스트레이션 |
|----------|------|---------------|
| **Entry Point Skill** | 사용자 진입점 | CLI 호출 + 세션 파일 생성 + Agent 디스패치 |
| **Agent** | 비즈니스 로직 | superpowers 조합 + 세션 파일 소비 + 결과 반환 |
| **Session File** | SKILL↔Agent 인터페이스 | 사전 추출된 스펙, 디버깅 가능한 중간 산출물 |

## Agents

| Agent | Superpowers 조합 | 역할 |
|-------|-----------------|------|
| `decompose` | (없음) | 대규모 스펙 → 모듈 분해 계획 (scope 판정 + path + req 분배) |
| `impl` | brainstorming (single 모드만) | 요구사항 분석 + CLAUDE.md/DEVELOPERS.md 생성 |
| `compiler` | test-driven-development | Inline TDD (Constraints → 테스트 → 구현 → 리팩토링) |
| `validator` | verification-before-completion | semantic drift 검출 (Requirements, Convention, DEVELOPERS.md) |
| `decompiler` | (없음) | 소스코드 → CLAUDE.md/DEVELOPERS.md 추출 |

## Commands

| Command | 역할 |
|---------|------|
| `/project-setup` | CLAUDE.md에 Instructions + Conventions 생성/업데이트 (convention-update 흡수) |
| `/migrate` | 버전 업그레이드 마이그레이션 (v6→v7, v9→v10 등) |

## Skills

### Core Skills (v10)

| Skill | 역할 |
|-------|------|
| `/impl` | 요구사항 → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| `/compile` | CLAUDE.md + DEVELOPERS.md → 소스코드 (Inline TDD from Constraints) |
| `/validate` | 문서-코드 일치 검증 (Deterministic CLI + semantic drift + auto-fix) |
| `/decompile` | 소스코드 → CLAUDE.md + DEVELOPERS.md 추출 |

### Phase 2 (Core 안정 후 추가 예정)

| Skill | 역할 |
|-------|------|
| `/bugfix` | 소스코드 버그 → 3계층 추적 → 수정 |
| `/impl-review` | CLAUDE.md 품질 리뷰 |
| `/impact` | 문서 변경 → 영향 모듈 분석 |
| `/diff-spec` | 문서 버전 간 시맨틱 diff |
| `/status` | 프로젝트 건강도 대시보드 |
| `/refactor` | 모듈 분할/병합 |

### CLI Subcommands (Rust Core)

| CLI | 역할 |
|-----|------|
| `scan-claude-md` | 기존 CLAUDE.md 인덱스 생성 |
| `diff-compile-targets` | 변경된 CLAUDE.md/DEVELOPERS.md 감지 |
| `resolve-boundary` | 바운더리 결정 |
| `analyze-code` | 코드 분석 (6개 언어) |
| `parse-claude-md` | CLAUDE.md 파싱 |
| `validate-schema` | 스키마 검증 |
| `format-exports` | Exports 마크다운 생성 |
| `format-analysis` | 분석 요약 마크다운 생성 |
| `validate-convention` | Conventions 섹션 검증 |
| `fix-schema` | 누락 섹션 자동 추가 |
| `contract-hash` | SHA-256 해시 (변경 감지) |
| `parse-tree` | 디렉토리 구조 파싱 |

## Invariants

### INV-1: 트리 구조 의존성
```
node.dependencies ⊆ node.children
```

### INV-2: Self-contained 바운더리
```
validate(node) = validate(node.claude_md, node.direct_files)
```

### INV-3: CLAUDE.md ↔ DEVELOPERS.md 쌍
```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
--strict 모드에서 DEVELOPERS.md 부재를 warning으로 보고
```

### INV-4: 업데이트 책임
```
/impl → CLAUDE.md + DEVELOPERS.md (문서 정의)
/compile → Source Code (문서 기반 코드 생성, 문서 읽기 전용)
/decompile → CLAUDE.md + DEVELOPERS.md (코드에서 문서 추출)
/validate → 위반 보고 + 대화형 해소 (사용자 승인)
```

### INV-5: Conventions 섹션 배치 규칙
```
project_root/CLAUDE.md MUST contain ## Conventions (6 required subsections)
module_root/CLAUDE.md MAY contain ## Conventions (override; 없으면 project_root에서 상속)
```

## Development Principles

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **Language-agnostic**: 파일 확장자 기반 자동 감지
3. **File-based results**: Agent 결과는 파일로 저장, 경로만 반환
4. **Simple retry**: 스키마 검증 1회, 테스트 재시도 3회
5. **Version management**: 변경 시 `.claude-plugin/plugin.json`의 `version` 필드를 반드시 bump

## Superpowers Coexistence

claude-md는 superpowers의 domain component들을 조합하여 "문서 기반 개발" 비즈니스를 만든다.

### 역할 분담

| 레이어 | 담당 | 도구 |
|--------|------|------|
| 스펙 정의·검증·추적 | claude-md | /impl, /validate, /decompile |
| 일괄 코드 재생성 | claude-md | /compile (batch) |
| 점진적 코드 작성 | superpowers | TDD (CLAUDE.md/DEVELOPERS.md 기반) |
| 프로세스 규율 | superpowers | brainstorming, plans, debugging, verification |

### 3-Layer 조합 구조

| Layer | 역할 | 구현 |
|-------|------|------|
| Layer 0 | 조합 설정 | /project-setup → `## Instructions` 자동 생성 (Claude Code 자동 로드) |
| Layer 1 | 스펙 추출 | SKILL → 세션 파일 생성 → Agent dispatch |
| Layer 2 | 순수 실행 | Agent가 세션 파일 + Skill(superpowers:xxx) 조합으로 실행 |

### Agent-Level 조합

| Agent | Superpowers Component | 조합 방식 |
|-------|----------------------|----------|
| impl | brainstorming | 요구사항 탐색/설계 전 brainstorming 로드 |
| compiler | test-driven-development | Constraints → TDD Red-Green-Refactor |
| validator | verification-before-completion | 증거 기반 검증 규율 |
| decompiler | (없음) | 추출 작업, 프로세스 규율 불필요 |
