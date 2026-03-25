# claude-md-plugin

## Purpose

**Source Code가 SSOT, CLAUDE.md는 사전학습 인덱스 + 인간 지식 저장소.**

코드베이스의 사전학습된 이해(index)와 코드에 없는 지식(metadata)을 구조화하여,
AI와 인간이 코드를 더 빠르고 정확하게 이해하고 수정할 수 있게 합니다.

## Core Philosophy

```
┌──────────────────────────────────────────────────────────────┐
│                    claude-md-plugin v6                        │
│                                                              │
│   Source Code (SSOT)                                         │
│         │                                                    │
│         ├──── /decompile ──→ CLAUDE.md + DEVELOPERS.md 추출  │
│         ├──── /validate ──→  문서-코드 일치 검증             │
│         │                                                    │
│   CLAUDE.md (pre-learning index + human knowledge)           │
│         │                                                    │
│         ├──── /impl ──→     요구사항 → CLAUDE.md 정의        │
│         ├──── /compile ──→  CLAUDE.md 기반 코드 생성         │
│         └──── /bugfix ──→   3계층 추적 → 수정               │
└──────────────────────────────────────────────────────────────┘
```

| 개념 | 역할 | 설명 |
|------|------|------|
| **Source of Truth** | Source Code | 실제 구현, 유일한 진실 |
| **Pre-learning Index** | CLAUDE.md | 코드 이해 가속을 위한 사전학습 인덱스 |
| **Human Knowledge** | CLAUDE.md | 코드에 없는 인간 지식 (제약, 맥락, 컨벤션) |
| **Deep Context** | DEVELOPERS.md | WHY — 결정 근거, 불변식 배경, 운영 맥락 |
| **Auto Index** | .claude/index.md | 코드에서 자동 추출한 인터페이스/동작 인덱스 (planned) |

**불일치 시**: 문서를 업데이트한다 (코드가 SSOT).

## 3-Document System

```
module/
├── CLAUDE.md              ← Human-authored / Auto-loaded / 200-600 tok
│   Critical ND-E. 코드 수정 시 즉시 알아야 할 규칙과 맥락.
│   Claude Code가 계층적으로 자동 로드.
│
├── DEVELOPERS.md          ← Human-authored / On-demand / 선택적
│   Local ND-E. 깊은 이해를 위한 맥락 (WHY).
│   CLAUDE.md Instructions + 플러그인 명령어로 로드 보장.
│
└── .claude/
    └── index.md           ← Auto-generated (planned) / On-demand
        P + ND-D. 코드에서 추출한 인터페이스/동작/구조 인덱스.
        /sync로 생성/갱신 (planned). 인간 편집 불가.
```

### CLAUDE.md Schema

| 섹션 | 존재 규칙 | None 허용 | 상속 |
|------|----------|----------|------|
| `## Purpose` | 항상 필수 | X | 없음 (각 모듈 고유) |
| `## Constraints` | 항상 필수 | O | **없음 (자기완결)** — 상위 제약 포함 반복 |
| `## Domain Context` | 항상 필수 | O | 없음 (로컬 맥락) |
| `## Conventions` | project/module root 필수 | X | **있음 (override)** — 부모와 다른 부분만 작성 |
| `## Instructions` | **project root only** | X | project root에서 전역 적용 |

### DEVELOPERS.md Schema

| 섹션 | 필수 | None 허용 | 내용 |
|------|------|----------|------|
| `## Domain Context` | O | O | 결정 근거, 상세 제약 배경 |
| `## Invariants` | O | O | 비즈니스 불변식 + 근거 |
| `## Decision Log` | O | O | ADR 스타일: 맥락/결정/근거 |
| `## Operations` | O | O | Gotchas, 배포, 모니터링 |
| `## File Map` | O | O | 파일별 역할 및 관계 |

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

> /decompile, /compile, /validate는 SKILL.md가 v5 전제로 deprecated.
> Agent/reference는 v6 완료. Phase 2에서 SKILL.md 재설계 후 복원 예정.

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
│ 6. CLAUDE.md 생성                           │
│ 7. DEVELOPERS.md 생성                       │
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
│ CLAUDE.md 생성 (WHAT)                       │
│ Bash(claude-md-core validate-schema)        │
└─────────────────────────────────────────────┘
```

#### /compile (CLAUDE.md → 소스코드)

```
User: /compile [--all] [--integration] [--dry-run]
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point)                 │
│                                             │
│ 0. --all 분기                               │
│    ├─ YES → 모든 CLAUDE.md 검색             │
│    └─ NO  → Bash(diff-compile-targets)      │
│             변경 감지                        │
│             targets = 0 → 종료              │
│ 1. 대상 CLAUDE.md 필터                      │
│ 2. compile-context 존재 확인 (optional)     │
│ 3. 언어 자동 감지                           │
│ 4. 의존성 그래프 기반 실행 (leaf-first)     │
│    같은 depth 독립 모듈은 병렬,             │
│    의존 관계는 순차 처리                    │
│    Task(test-designer) → Task(compiler)     │
│    실패 시 피드백 루프 (최대 1회)           │
│                                             │
│ ⚠ DEPRECATED: test-designer가 v5 섹션에    │
│   의존하여 현재 실행 불가. Phase 2에서 재설계│
└─────────────────────────────────────────────┘
```

#### /validate (문서-코드 일치 검증)

```
User: /validate
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL (Entry Point)                │
│                                             │
│ 1. Bash(validate-schema) → 스키마 검증      │
│ 2. Task(validator) 배치 병렬 → Drift 검증   │
│ 3. 중간 결과 확인 (이슈 있는 디렉토리 선별) │
│ 4. Task(issue-verifier) 배치 병렬 → 재검증  │
│ 5. Task(violation-reporter) 배치 → 위반 보고│
│ 6. 통합 보고서 생성                         │
│                                             │
│ ⚠ DEPRECATED: issue-verifier/violation-    │
│   reporter가 v5 섹션에 의존하여 실행 불가.  │
│   validator만 v6 호환. Phase 2에서 재설계   │
└────────────────────┬────────────────────────┘
                     ▼
┌──────────────────────────────────────────┐
│ validator (v6 호환, drift 검증)          │
│ Constraints / Domain Context /           │
│ Convention / DEVELOPERS.md / Boundary    │
└──────────────────────────────────────────┘
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
│ Phase 4: Task(debug-layer-analyzer, L2)     │
│ Phase 5: Task(debug-layer-analyzer, L3)     │
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
│ 1. 인자 파싱 (request + --path)             │
│ 2. 의도 분류 (FEATURE/BUGFIX/COMPILE/       │
│    VALIDATE/AMBIGUOUS)                      │
│ 3. CLAUDE.md 존재 확인 (FEATURE 제외)       │
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
| `impl` | active | 요구사항 분석 및 CLAUDE.md + DEVELOPERS.md 생성 |
| `dep-explorer` | active | 의존성 탐색 (requirement 모드: 새 모듈 의존성, module 모드: 기존 모듈 의존자) |
| `decompiler` | active | 소스코드에서 CLAUDE.md + DEVELOPERS.md 추출 |
| `compiler` | active | CLAUDE.md Constraints + Domain Context 기반 소스코드 생성 (GREEN + REFACTOR) |
| `debug-layer-analyzer` | active | 단일 계층(L1/L2/L3) 진단 분석 (debugger의 sub-agent) |
| `debugger` | active | 소스코드 런타임 버그 → 3계층 추적 → 수정 (orchestrator) |
| `impl-reviewer` | active | CLAUDE.md 품질 리뷰 및 요구사항 커버리지 검증 |
| `validator` | active | CLAUDE.md Constraints/Domain Context/Convention drift 검증 |

## Commands

| Command | 역할 |
|---------|------|
| `/dev` | 자연어 요청 분류 → 스킬 라우팅 |
| `/project-setup` | CLAUDE.md에 Conventions 섹션 생성 |
| `/convention-update` | CLAUDE.md Conventions 섹션 업데이트 |
| `/migrate` | 버전 업그레이드 시 CLAUDE.md 스키마 마이그레이션 |

## Skills

### Entry Point Skills

| Skill | 상태 | 역할 |
|-------|------|------|
| `/impl` | active | 요구사항 → CLAUDE.md |
| `/decompile` | **deprecated** | 소스코드 → CLAUDE.md + DEVELOPERS.md (SKILL.md가 v5 전제, Phase 2 재설계 예정) |
| `/compile` | **deprecated** | CLAUDE.md → 소스코드 (test-designer 의존, Phase 2 재설계 예정) |
| `/validate` | **deprecated** | 문서-코드 일치 검증 (issue-verifier/violation-reporter 의존, Phase 2 재설계 예정) |
| `/bugfix` | active | 소스코드 런타임 버그 → 3계층 추적 → 수정 |
| `/impl-review` | active | CLAUDE.md 품질 리뷰 |
| `/impact` | planned | 문서 변경 → 영향받는 모듈 분석 |
| `/diff-spec` | planned | 문서 버전 간 시맨틱 diff |
| `/status` | planned | 프로젝트 건강도 대시보드 |
| `/refactor` | planned | 모듈 분할/병합 (문서 수준 리팩토링) |
| `/resolve` | planned | /validate 위반 해소 |

### Internal Skills & CLI Subcommands

| Skill | 타입 | 역할 |
|-------|------|------|
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `scan-claude-md` | CLI | 기존 CLAUDE.md 인덱스 생성 |
| `diff-compile-targets` | CLI | 변경된 CLAUDE.md 감지 (incremental compile) |
| `resolve-boundary` | CLI | 바운더리 결정 |
| `analyze-code` | CLI | 코드 분석 |
| `parse-claude-md` | CLI | CLAUDE.md 파싱 |
| `validate-schema` | CLI | 스키마 검증 |
| `format-exports` | CLI | analyze-code JSON → Exports 마크다운 생성 |
| `format-analysis` | CLI | analyze-code JSON → 분석 요약 마크다운 생성 |
| `validate-convention` | CLI | Conventions 섹션 검증 |
| `fix-schema` | CLI | 누락된 allow-none 섹션 자동 추가 |
| `index-project` | CLI | 프로젝트 전체 인덱싱 |
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
/impl → CLAUDE.md + DEVELOPERS.md (문서 정의)
/compile → Source Code (CLAUDE.md 기반 코드 생성, CLAUDE.md 읽기 전용)
/decompile → CLAUDE.md + DEVELOPERS.md (코드에서 문서 추출)
/bugfix → Source Code 재생성 (기본) / CLAUDE.md 수정 (사용자 승인 필수, L1 root cause)
/impl-review → CLAUDE.md (사용자 승인 후 fix patch)
/validate → 위반 보고 (문서 수정 안 함)
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
