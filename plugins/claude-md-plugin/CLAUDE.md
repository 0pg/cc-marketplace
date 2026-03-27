# claude-md-plugin

## Purpose

**CLAUDE.md가 Primary SSOT — PM의 요구사항 문서.**

비즈니스 요구사항을 CLAUDE.md로 정의하고, DEVELOPERS.md로 시스템 레벨로 구체화하여,
소스코드를 파생 산출물로 생성하는 문서-코드 동기화 플러그인.

## Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v7                        │
│                                                              │
│   CLAUDE.md (Primary SSOT — PM 요구사항)                     │
│         │                                                    │
│         ├──── /impl ──→     요구사항 → CLAUDE.md 정의        │
│         ├──── /compile ──→  CLAUDE.md 기반 코드 생성         │
│         ├──── /validate ──→ 문서-코드 일치 검증              │
│         └──── /bugfix ──→   3계층 추적 → 수정               │
│                                                              │
│   DEVELOPERS.md (Derived Spec — 개발자 명세)                 │
│         │                                                    │
│         └──── Constraints = 테스트 생성 원천                 │
│                                                              │
│   Source Code (Derived Artifact)                             │
│         │                                                    │
│         └──── /decompile ──→ CLAUDE.md + DEVELOPERS.md 추출  │
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

**컨벤션 우선순위** (module_root != project_root인 경우):
1. module_root CLAUDE.md `## Conventions` (override)
2. project_root CLAUDE.md `## Conventions` (default)
3. project_root CLAUDE.md 일반 내용 (최종 fallback)

### 트리 구조 의존성
- **부모 → 자식**: 참조 가능
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

각 CLAUDE.md는 자신의 바운더리 내에서 self-contained여야 합니다.

## Architecture

### Active Workflows

#### /impl (요구사항 → CLAUDE.md)

```
User: /impl "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl SKILL (Entry Point)                    │
│                                             │
│ 1. Bash(scan-claude-md) → 기존 CLAUDE.md    │
│    인덱스 생성                              │
│ 2. Task(impl) + claude_md_index_file        │
│    → CLAUDE.md + DEVELOPERS.md 작성         │
│ 3. git diff → 변경사항 Diff 표시            │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ impl AGENT                                  │
│                                             │
│ 1. 요구사항 분석                            │
│ 2. Task(dep-explorer) → 의존성 탐색         │
│ 3. AskUserQuestion → 모호한 부분 명확화     │
│ 4. 대상 경로 결정                           │
│ 5. 기존 CLAUDE.md 병합 (필요시)             │
│ 6. CLAUDE.md 생성 (Requirements)            │
│ 7. DEVELOPERS.md 생성 (Constraints)         │
│ 8. Bash(claude-md-core validate-schema) → 검증│
└─────────────────────────────────────────────┘
```

#### /decompile (소스코드 → CLAUDE.md)

```
User: /decompile
        │
        ▼
┌─────────────────────────────────────────────┐
│ decompile SKILL (Entry Point)               │
│                                             │
│ 1. Skill("tree-parse") → 대상 목록          │
│ 2. For each directory (leaf-first):         │
│    Task(decompiler) 호출                    │
│    git diff → 추출 문서 Diff 표시           │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ decompiler AGENT                            │
│                                             │
│ Bash(claude-md-core resolve-boundary)       │
│ Bash(claude-md-core analyze-code)           │
│ AskUserQuestion → 불명확한 부분 질문        │
│ CLAUDE.md 생성 (Requirements)               │
│ DEVELOPERS.md 생성 (Constraints)            │
│ Bash(claude-md-core validate-schema)        │
└─────────────────────────────────────────────┘
```

#### /compile (CLAUDE.md → 소스코드)

```
User: /compile [--all] [--conflict skip|overwrite] [--dry-run] [--validate]
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point)                 │
│                                             │
│ 0. --all 분기                               │
│    ├─ YES → 모든 CLAUDE.md 검색             │
│    └─ NO  → Bash(diff-compile-targets)      │
│             변경 감지 (CLAUDE.md +           │
│             DEVELOPERS.md 모두 트리거)       │
│             targets = 0 → 종료              │
│ 1. 대상 CLAUDE.md 필터                      │
│ 2. compile-context 존재 확인 (optional)     │
│ 3. 언어 자동 감지                           │
│ 4. 의존성 그래프 기반 실행 (leaf-first)     │
│    같은 depth 독립 모듈은 병렬,             │
│    의존 관계는 순차 처리                    │
│    Task(compiler) — Inline TDD             │
│    (DEVELOPERS.md Constraints → 테스트 →   │
│     GREEN → REFACTOR)                       │
└─────────────────────────────────────────────┘
```

#### /validate (문서-코드 일치 검증)

```
User: /validate [path] [--strict]
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL (Entry Point)                │
│                                             │
│ 1. Glob → CLAUDE.md 수집                    │
│ 2. Deterministic 검증 (CLI only)            │
│    2a. validate-schema + fix-schema         │
│    2b. validate-convention → 구조 검증      │
│    2c. resolve-boundary → INV-1 검증        │
│ 3. Semantic 검증 (validator agent)          │
│    Task(validator) 배치 병렬                │
│    (3 카테고리: Requirements,               │
│     Convention CODE_VIOLATION,              │
│     DEVELOPERS.md)                          │
│ 4. 통합 보고서 (Phase 2 + 3 병합)          │
└─────────────────────────────────────────────┘
```

#### /bugfix (소스코드 버그 → 3계층 추적 → 수정)

```
User: /bugfix [--error "..."] [--test "..."]
        │
        ▼
┌─────────────────────────────────────────────┐
│ bugfix SKILL (Entry Point)                  │
│                                             │
│ 1. Bug Report 수집 (에러/테스트 정보)       │
│ 2. 입력 타입 분류 (기술적 에러/테스트/기능) │
│ 3. CLAUDE.md + DEVELOPERS.md 존재 확인      │
│ 4. 사전 검증 (스키마/미컴파일 변경)         │
│ 5. Task(debugger) → 진단                   │
│    ├─ L3 root cause → /compile 재실행      │
│    └─ L1 root cause → 사용자 승인 후       │
│       문서 수정 → /compile 재실행           │
│ 6.5. git diff → 수정사항 Diff 표시         │
│ 7. Skill("claude-md-plugin:compile")        │
│ 8. 검증 (원본 테스트 재실행)                │
│ 9. 결과 보고                                │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ debugger AGENT (Orchestrator)               │
│                                             │
│ Phase 1-2: 에러 재현 + 파싱 (inline)        │
│ Phase 2.5: CLI → 파일 저장 (context 0)      │
│ Phase 3: Task(debug-layer-analyzer, L1)     │
│          L1 = CLAUDE.md Requirements        │
│ Phase 4: Task(debug-layer-analyzer, L2)     │
│          L2 = DEVELOPERS.md Constraints     │
│ Phase 5: Task(debug-layer-analyzer, L3)     │
│          L3 = Source Code                   │
│ Phase 6: Findings Read → 교차 분석          │
│ Phase 6.5: L1 root cause → 사용자 승인      │
│ Phase 7: Fix 제안 + 사용자 승인 + Edit      │
└─────────────────────────────────────────────┘
```

#### /impl-review (CLAUDE.md 품질 리뷰)

```
User: /impl-review [path]
        │
        ▼
┌─────────────────────────────────────────────┐
│ impl-review SKILL (Entry Point)             │
│                                             │
│ 1. 인자 파싱 & 대상 해석                    │
│ 2. Bash(claude-md-core validate-schema)     │
│ 3. Task(impl-reviewer) → 3차원 리뷰        │
│ 3.5. git diff → 수정 제안 Diff 표시        │
└─────────────────────────────────────────────┘
```

### /dev (자연어 → 스킬 라우팅)

```
User: /dev "request"
        │
        ▼
┌─────────────────────────────────────────────┐
│ dev COMMAND                                 │
│                                             │
│ 0. superpowers 공존 확인                    │
│    (superpowers:brainstorming 존재 시       │
│     안내 메시지 출력 → 종료)                │
│ 1. 인자 파싱 (request + --path)             │
│ 2. 의도 분류 (FEATURE/BUGFIX/COMPILE/       │
│    DECOMPILE/VALIDATE/RESOLVE/              │
│    IMPACT/DIFF/STATUS/REFACTOR/AMBIGUOUS)   │
│ 3. CLAUDE.md 존재 확인                       │
│    (FEATURE+DECOMPILE 제외)                 │
│    없으면 → 안내 후 종료                    │
│ 4. Skill(target) 호출                       │
└─────────────────────────────────────────────┘
```

### 설계 원칙

| 컴포넌트 | 역할 | 오케스트레이션 |
|----------|------|---------------|
| **Entry Point Skill** | 사용자 진입점 | 간단 (파일 검색, 반복, Agent 호출) |
| **Internal Skill** | 단일 기능 (SRP) | 없음, Stateless |
| **Agent** | 비즈니스 로직 | 복잡 (N개 Skill, 재시도, 상태) |

## Agents

| Agent | 상태 | 역할 |
|-------|------|------|
| `impl` | active | 요구사항 분석 및 CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) 생성 |
| `dep-explorer` | active | 의존성 탐색 (requirement 모드: 새 모듈 의존성, module 모드: 기존 모듈 의존자) |
| `decompiler` | active | 소스코드에서 CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) 추출 |
| `compiler` | active | DEVELOPERS.md Constraints 기반 Inline TDD (테스트 생성 → GREEN → REFACTOR) |
| `debug-layer-analyzer` | active | 단일 계층(L1/L2/L3) 진단 분석 (debugger의 sub-agent) |
| `debugger` | active | 소스코드 런타임 버그 → 3계층 추적 → 수정 (orchestrator) |
| `impl-reviewer` | active | CLAUDE.md 품질 리뷰 및 요구사항 커버리지 검증 |
| `validator` | active | CLAUDE.md Requirements/Convention CODE_VIOLATION/DEVELOPERS.md semantic drift 검증 |

## Commands

| Command | 역할 |
|---------|------|
| `/dev` | 자연어 요청 분류 → 스킬 라우팅 |
| `/project-setup` | CLAUDE.md에 Conventions 섹션 생성 |
| `/convention-update` | CLAUDE.md Conventions 섹션 업데이트 |
| `/migrate` | 버전 업그레이드 시 CLAUDE.md 스키마 마이그레이션 (v6→v7 포함) |

## Skills

### Entry Point Skills

| Skill | 상태 | 역할 |
|-------|------|------|
| `/impl` | active | 요구사항 → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) |
| `/decompile` | active | 소스코드 → CLAUDE.md + DEVELOPERS.md |
| `/compile` | active | CLAUDE.md + DEVELOPERS.md → 소스코드 (Inline TDD from Constraints) |
| `/validate` | active | 문서-코드 일치 검증 (Deterministic CLI + 3 semantic drift) |
| `/bugfix` | active | 소스코드 런타임 버그 → 3계층 추적 → 수정 |
| `/impl-review` | active | CLAUDE.md 품질 리뷰 |
| `/status` | active | 프로젝트 건강도 대시보드 |
| `/impact` | active | 문서 변경 → 영향받는 모듈 분석 (Requirements 기반) |
| `/diff-spec` | active | 문서 버전 간 시맨틱 diff |
| `/resolve` | active | /validate 위반 대화형 해소 |
| `/refactor` | active | 모듈 분할/병합 (Requirements 그루핑 기반) |

### Internal Skills & CLI Subcommands

| Skill | 타입 | 역할 |
|-------|------|------|
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `scan-claude-md` | CLI | 기존 CLAUDE.md 인덱스 생성 |
| `diff-compile-targets` | CLI | 변경된 CLAUDE.md/DEVELOPERS.md 감지 (incremental compile) |
| `resolve-boundary` | CLI | 바운더리 결정 |
| `analyze-code` | CLI | 코드 분석 |
| `parse-claude-md` | CLI | CLAUDE.md 파싱 |
| `validate-schema` | CLI | 스키마 검증 |
| `format-exports` | CLI | analyze-code JSON → Exports 마크다운 생성 |
| `format-analysis` | CLI | analyze-code JSON → 분석 요약 마크다운 생성 |
| `validate-convention` | CLI | Conventions 섹션 검증 |
| `fix-schema` | CLI | 누락된 allow-none 섹션 자동 추가 |
| `contract-hash` | CLI | CLAUDE.md 전체 파일 SHA-256 해시 (변경 감지용) |

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
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
--strict 모드에서 DEVELOPERS.md 부재를 warning으로 보고
```

### INV-4: 업데이트 책임
```
/impl → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) (문서 정의)
/compile → Source Code (CLAUDE.md + DEVELOPERS.md 기반 코드 생성, 문서 읽기 전용)
/decompile → CLAUDE.md (Requirements) + DEVELOPERS.md (Constraints) (코드에서 문서 추출)
/bugfix → Source Code 재생성 (기본) / CLAUDE.md 수정 (사용자 승인 필수, L1 root cause)
/impl-review → CLAUDE.md (사용자 승인 후 fix patch)
/validate → 위반 보고 (문서 수정 안 함)
/resolve → CLAUDE.md 또는 Source Code (사용자 선택에 따라 drift 해소)
/impact → 분석 보고 (문서 수정 안 함)
/diff-spec → 분석 보고 (문서 수정 안 함)
/status → 분석 보고 (문서 수정 안 함)
/refactor → CLAUDE.md + DEVELOPERS.md (문서 분할/병합)
```

### INV-5: Conventions 섹션 배치 규칙
```
project_root/CLAUDE.md MUST contain ## Conventions (6 required subsections)
module_root/CLAUDE.md MAY contain ## Conventions (override; 없으면 project_root에서 상속)
싱글 모듈: project_root == module_root → 같은 CLAUDE.md에 배치
```

## Development Principles

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **Language-agnostic**: 파일 확장자 기반 자동 감지
3. **File-based results**: Agent 결과는 파일로 저장, 경로만 반환
4. **Simple retry**: 스키마 검증 1회, 테스트 재시도 3회
5. **Version management**: 변경 시 `.claude-plugin/plugin.json`의 `version` 필드를 반드시 bump

## Superpowers Coexistence

claude-md-plugin은 superpowers 플러그인(v4.x)과 프로세스/도메인 레이어로 공존할 수 있다.

### 역할 분담

| 관심사 | 담당 |
|--------|------|
| 프로세스 강제 (HARD-GATE, Iron Laws) | superpowers |
| Worktree 격리 | superpowers |
| 2-stage 리뷰 | superpowers |
| 문서 SSOT, 코드 생성 | claude-md |
| 문서-코드 동기화 검증 | claude-md |
| 도메인 특화 디버깅 | claude-md |

### 감지 메커니즘

`/dev` 라우터가 로드된 스킬 목록에서 `superpowers:brainstorming` 존재를 확인.
활성 시 라우팅을 비활성화하고 superpowers 프로세스 흐름으로 안내.

### 소비자 프로젝트 권장 Instructions

superpowers + claude-md를 함께 사용하는 프로젝트의 `## Instructions`에 권장:

```markdown
## Instructions
- 소스코드는 /compile로만 생성. Write tool로 직접 소스 파일 생성 금지.
- 모든 코드 변경은 CLAUDE.md 수정 → /compile 순서.
- writing-plans의 태스크는 Skill 호출 단위로 작성 (/impl, /compile, /validate).
- 완료 선언 전 /validate --strict 실행 필수.
```
