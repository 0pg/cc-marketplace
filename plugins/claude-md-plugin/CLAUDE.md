# claude-md-plugin

## Purpose

**CLAUDE.md를 Source of Truth로 사용하여 문서-코드 동기화를 구현하는 플러그인.**

기존 접근법(소스코드 → 문서)을 역전시켜 CLAUDE.md가 명세가 되고, 소스코드가 산출물이 되는 패러다임을 제공합니다.

## Core Philosophy: Compile/Decompile 패러다임

**CLAUDE.md + IMPLEMENTS.md는 소스코드이고, 소스코드는 바이너리다.**

```
┌─────────────────────────────────────────────────────────────┐
│                    전통적 소프트웨어                          │
│                                                             │
│   .h (헤더)  +  .c (소스)  ─── compile ──→  Binary (.exe)   │
│   Binary (.exe)  ─── decompile ──→  .h + .c                 │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin (듀얼 문서 시스템)       │
│                                                             │
│   CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)                    │
│         │                                                   │
│         └──── /compile ──→  Source Code (구현)              │
│                                                             │
│   Source Code (구현)  ─── /decompile ──→                    │
│         └──→ CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)         │
└─────────────────────────────────────────────────────────────┘
```

| 전통적 개념 | claude-md-plugin | 역할 |
|------------|------------------|------|
| .h (헤더) | CLAUDE.md | WHAT - 인터페이스, 스펙 |
| .c (소스) | IMPLEMENTS.md | HOW - 구현 명세 |
| Binary | Source Code (.ts, .py, ...) | 실행물 |
| **compile** | CLAUDE.md + IMPLEMENTS.md → Source Code | `/compile` |
| **decompile** | Source Code → CLAUDE.md + IMPLEMENTS.md | `/decompile` |

**왜 이 비유인가?**
- **CLAUDE.md** (.h)는 "무엇을(WHAT)" 정의 → 인터페이스, PRD
- **IMPLEMENTS.md** (.c)는 "어떻게(HOW)" 구현하는지 정의 → 알고리즘, 상수, 에러처리
- **Source Code**는 기계가 실행하는 것 → 실제 소스코드 (런타임이 실행)
- **Compile**은 스펙+구현명세에서 실행 가능한 형태로 변환
- **Decompile**은 실행 가능한 형태에서 스펙+구현명세 추출

## 핵심 개념

### CLAUDE.md = 소스코드의 스펙
각 디렉토리의 CLAUDE.md만으로:
- 어떤 파일들이 존재해야 하는지
- 각 파일이 어떤 인터페이스를 제공하는지
- 어떤 동작을 해야 하는지

를 알 수 있어야 합니다.

**섹션 구성 (7 required + 3 optional)**:

| 섹션 | 필수 | 설명 |
|------|------|------|
| Purpose | required | 디렉토리의 책임 (1-2문장) |
| Summary | required | 모듈의 역할/기능 요약 |
| Exports | required | public interface 카탈로그 (`None` 허용) |
| Behavior | required | 동작 시나리오 (input → output) (`None` 허용) |
| Contract | required | 사전/사후 조건, 불변식 (`None` 허용) |
| Protocol | required | 상태 전이, 호출 순서 (`None` 허용) |
| Domain Context | required | compile 재현을 위한 맥락 (`None` 허용) |
| Structure | optional | 하위 디렉토리/파일이 있을 때 |
| Dependencies | optional | 외부/내부 의존성이 있을 때 |
| Constraints | optional | 제약사항이 있을 때 |

> SSOT: `skills/schema-validate/references/schema-rules.yaml`

### 트리 구조 의존성
- **부모 → 자식**: 참조 가능
- **자식 → 부모**: 참조 불가
- **형제 ↔ 형제**: 참조 불가

각 CLAUDE.md는 자신의 바운더리 내에서 self-contained여야 합니다.

### CLAUDE.md Exports = Interface Catalog

**Exports 섹션은 다른 모듈이 코드 탐색 없이 인터페이스를 파악할 수 있는 카탈로그입니다.**

| 시나리오 | Exports 섹션 활용 |
|----------|------------------|
| **생성 시** | 모든 public interface를 시그니처 레벨로 명시 |
| **참조 시** | 의존 모듈의 Exports 섹션으로 인터페이스 파악 (코드 탐색 불필요) |
| **변경 시** | Exports 변경 = Breaking Change, 참조하는 모듈 확인 필요 |

```
의존 모듈 참조 시 탐색 순서:
1. 의존 모듈 CLAUDE.md Exports ← 여기서 인터페이스 확인
2. 의존 모듈 CLAUDE.md Behavior ← 동작 이해 필요 시
3. 실제 소스코드 ← 최후 수단 (Exports로 불충분할 때만)
```

### Domain Context = 맥락 카탈로그 (CLAUDE.md)

**Domain Context 섹션은 "왜?" 이 결정을 했는지 설명하는 맥락 정보입니다.**

| 역할 | 설명 |
|------|------|
| **자체 compile 재현** | 해당 CLAUDE.md → 동일한 코드 |
| **의존자 compile 영향** | 이 모듈을 참조하는 다른 모듈의 compile 결정에 필요한 맥락 |

### IMPLEMENTS.md = 구현 명세 (듀얼 문서 시스템)

**IMPLEMENTS.md는 CLAUDE.md와 1:1로 매핑되는 "어떻게(HOW)" 문서입니다.**

```
auth/
├── CLAUDE.md       ← WHAT (스펙)
│   ├── Exports: validateToken(token: string): Claims
│   └── Domain Context: 토큰 만료 7일 (PCI-DSS)
│
└── IMPLEMENTS.md   ← HOW (구현 명세)
    ├── [Planning Section] - /spec이 업데이트
    │   ├── Architecture Decisions
    │   ├── Module Integration Map
    │   ├── External Dependencies
    │   ├── Implementation Approach
    │   └── Technology Choices
    │
    └── [Implementation Section] - /compile이 업데이트
        ├── Algorithm
        ├── Key Constants
        ├── Error Handling
        ├── State Management
        └── Implementation Guide
```

**명령어별 업데이트 범위:**

| 명령어 | CLAUDE.md | IMPLEMENTS.md |
|--------|-----------|---------------|
| `/spec` | 생성/업데이트 | Planning Section 업데이트 |
| `/compile` | 읽기 전용 | Implementation Section 업데이트 |
| `/decompile` | 생성 (전체) | 생성 (전체 - Planning + Implementation) |

**Exports = 인터페이스 카탈로그, Domain Context = 맥락 카탈로그, IMPLEMENTS.md = 구현 명세**

### Schema v2 (Cross-Reference & Diagrams)

v2는 CLAUDE.md를 **machine-readable, cross-referenceable** 스펙 시스템으로 진화시킵니다.

| v1 (암묵적) | v2 (명시적) | 효과 |
|------------|------------|------|
| 버전 마커 없음 | `<!-- schema: 2.0 -->` | 파서가 버전별 분기 가능 |
| 불릿 기반 Exports | `#### symbolName` 헤딩 | GitHub 앵커 → cross-reference |
| 평면적 Behavior | Actors + UC-N + Include/Extend | UseCase 다이어그램 자동 생성 |
| 참조 방법 없음 | `path/CLAUDE.md#symbol` | go-to-definition, find-references |

**v1/v2 하위 호환성**: v1 파일은 모든 명령어에서 정상 동작. v2 마이그레이션은 선택적 (`claude-md-core migrate`).

**SSOT 체인**: `schema-rules.yaml → build.rs (codegen) → Rust 상수 13개`

## Architecture

### /spec (요구사항 → CLAUDE.md + IMPLEMENTS.md)

```
User: /spec "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL (Entry Point)                    │
│                                             │
│ Task(spec-agent) → 요구사항 분석 및         │
│                    CLAUDE.md + IMPLEMENTS.md│
│                    작성 + 자동 리뷰         │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ spec-agent AGENT                            │
│                                             │
│ 1. 요구사항 분석                            │
│ 2. AskUserQuestion → 모호한 부분 명확화     │
│ 3. Task 정의 (상태 파일 저장)               │
│ 4. 대상 경로 결정                           │
│ 5. 기존 CLAUDE.md 병합 (필요시)             │
│ ┌─────────────────────────────────────────┐ │
│ │     ITERATION CYCLE (최대 3회)          │ │
│ │ 6. CLAUDE.md + IMPLEMENTS.md 생성       │ │
│ │ 7. Task(spec-reviewer) → 자동 리뷰      │ │
│ │ 8. approve → 다음 / feedback → 6으로   │ │
│ └─────────────────────────────────────────┘ │
│ 9. Skill("schema-validate") → 검증 (1회)    │
└─────────────────────────────────────────────┘
```

### /decompile (소스코드 → CLAUDE.md + IMPLEMENTS.md)

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
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ decompiler AGENT                            │
│                                             │
│ Skill("boundary-resolve") → 바운더리 분석   │
│ Skill("code-analyze") → 코드 분석           │
│ AskUserQuestion → 불명확한 부분 질문        │
│ CLAUDE.md 생성 (WHAT)                       │
│ IMPLEMENTS.md 생성 (HOW - 전체 섹션)        │
│ Skill("schema-validate") → 검증 (1회)       │
└─────────────────────────────────────────────┘
```

### /compile (CLAUDE.md + IMPLEMENTS.md → 소스코드)

```
User: /compile
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point + Orchestrator)  │
│                                             │
│ 1. 모든 CLAUDE.md + IMPLEMENTS.md 검색      │
│ 2. IMPLEMENTS.md 없으면 자동 생성           │
│ 3. 언어 자동 감지                           │
│ 4. For each pair:                           │
│                                             │
│  [RED] Task(compiler, phase=red)            │
│    └─ test_files + spec_json_path 반환      │
│              │                              │
│              ▼                              │
│  [TEST REVIEW] review_loop (최대 3회):      │
│    Task(test-reviewer) ← 독립 맥락          │
│    ├─ approve (score==100) → break          │
│    ├─ feedback → Task(compiler, phase=red)  │
│    │            피드백 기반 테스트 재생성     │
│    └─ else → warning, break                 │
│              │                              │
│              ▼                              │
│  [GREEN+REFACTOR]                           │
│    Task(compiler, phase=green-refactor)      │
│    └─ 기존 test_files 사용하여 구현          │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ compiler AGENT (phase별 실행)               │
│                                             │
│ [phase=red]                                 │
│   CLAUDE.md 읽기 → 테스트 생성만            │
│                                             │
│ [phase=green-refactor]                      │
│   기존 테스트 로드 → 구현 + 리팩토링        │
│   IMPLEMENTS.md Implementation Section 업데이트│
└─────────────────────────────────────────────┘
```

### /validate (문서-코드 일치 검증)

```
User: /validate
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL (Entry Point)                │
│                                             │
│ For each CLAUDE.md (병렬):                  │
│   Task(drift-validator)                     │
│   Task(export-validator)                    │
└─────────────────────────────────────────────┘
```

### 설계 원칙

| 컴포넌트 | 역할 | 오케스트레이션 |
|----------|------|---------------|
| **Entry Point Skill** | 사용자 진입점 | 간단 (파일 검색, 반복, Agent 호출) |
| **Internal Skill** | 단일 기능 (SRP) | 없음, Stateless |
| **Agent** | 비즈니스 로직 | 복잡 (N개 Skill, 재시도, 상태) |

## Agents

| Agent | 역할 |
|-------|------|
| `spec-agent` | 요구사항 분석 및 CLAUDE.md 생성 (자동 리뷰 사이클 포함) |
| `spec-reviewer` | CLAUDE.md/IMPLEMENTS.md 요구사항 충족 검증 (승인 조건: `score >= 80`, `req_coverage == 100%`, `schema_valid == passed`, `task_completion >= 80%`) |
| `decompiler` | 단일 디렉토리 CLAUDE.md + IMPLEMENTS.md 추출 |
| `recursive-decompiler` | 재귀적 디렉토리 탐색 + incremental 판단 + decompiler 오케스트레이션 |
| `compiler` | CLAUDE.md에서 소스코드 생성 (TDD, phase 지원: red/green-refactor/full) |
| `test-reviewer` | 생성된 테스트 코드의 스펙 커버리지 검증 (100% 기준) |
| `drift-validator` | CLAUDE.md-코드 일치 검증 |
| `export-validator` | Export 존재 검증 |
| `code-reviewer` | 코드 품질 + 컨벤션 검증 (code-convention.md 기반) |

## Skills

| Skill | 타입 | 역할 |
|-------|------|------|
| `/spec` | Entry Point | 요구사항 → CLAUDE.md |
| `/decompile` | Entry Point | 소스코드 → CLAUDE.md |
| `/compile` | Entry Point | CLAUDE.md → 소스코드 |
| `/validate` | Entry Point | 문서-코드 일치 검증 |
| `/project-setup` | Entry Point | 빌드/테스트 커맨드 → CLAUDE.md + code-convention.md |
| `/convention` | Entry Point | code-convention.md 조회/업데이트 |
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `boundary-resolve` | Internal | 바운더리 결정 |
| `code-analyze` | Internal | 코드 분석 |
| `claude-md-parse` | Internal | CLAUDE.md 파싱 |
| `schema-validate` | Internal | 스키마 검증 |
| `dependency-graph` | Internal | 모듈 의존성 그래프 생성 |
| `git-status-analyzer` | Internal | uncommitted 스펙 파일 찾기 |
| `commit-comparator` | Internal | 스펙 vs 소스 커밋 시점 비교 |
| `interface-diff` | Internal | Exports 시그니처 변경 감지 |
| `dependency-tracker` | Internal | 의존 모듈 영향 분석 |

## Core Rust Modules (core/src/)

| 모듈 | 역할 | CLI 명령어 |
|------|------|-----------|
| `tree_parser.rs` | 디렉토리 트리 스캔 | `parse-tree` |
| `boundary_resolver.rs` | 바운더리 결정 | `resolve-boundary` |
| `schema_validator.rs` | 스키마 검증 (v1/v2 듀얼) | `validate-schema` |
| `claude_md_parser.rs` | CLAUDE.md 파싱 (v1/v2 듀얼) | `parse-claude-md` |
| `dependency_graph.rs` | 모듈 의존성 그래프 | `dependency-graph` |
| `auditor.rs` | 완성도 감사 | `audit` |
| `code_analyzer.rs` | 소스코드 분석 | - |
| `symbol_index.rs` | 크로스 모듈 심볼 인덱싱 (v2) | `symbol-index` |
| `tree_utils.rs` | TreeParser/Auditor 공통 디렉토리 스캐너 | - |
| `diagram_generator.rs` | Mermaid 다이어그램 생성 (v2) | `generate-diagram --type usecase\|state\|component` |
| `implements_md_parser.rs` | IMPLEMENTS.md 파싱 | `parse-implements-md` |
| `migrator.rs` | v1→v2 자동 마이그레이션 (v2) | `migrate` |

## 불변식

### INV-1: 트리 구조 의존성
```
node.dependencies ⊆ node.children
```

### INV-2: Self-contained 바운더리
```
validate(node) = validate(node.claude_md, node.implements_md, node.direct_files)
```

### INV-3: CLAUDE.md ↔ IMPLEMENTS.md 쌍
```
∀ CLAUDE.md ∃ IMPLEMENTS.md (1:1 mapping)
path(IMPLEMENTS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'IMPLEMENTS.md')
```

### INV-4: Section 업데이트 책임
```
/spec → CLAUDE.md + IMPLEMENTS.md.PlanningSection
/compile → IMPLEMENTS.md.ImplementationSection
/decompile → CLAUDE.md + IMPLEMENTS.md.* (전체)
```

### INV-5: code-convention.md 업데이트 책임
```
/project-setup → code-convention.md (최초 생성)
/convention → code-convention.md (재분석/수동 수정)
/compile, /validate → code-convention.md (읽기 전용)
```

## 개발 원칙

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **언어 무관**: 파일 확장자 기반 자동 감지
3. **파일 기반 결과 전달**: Agent 결과는 파일로 저장, 경로만 반환
4. **단순한 재시도**: 스키마 검증 1회, 테스트 재시도 3회

## 문서 작성 규칙

### 의사코드 가이드

Agent/Skill 문서에서 로직 설명 시 의사코드 사용을 권장합니다.

| 허용 | 비허용 |
|------|--------|
| 언어 중립적 의사코드 | 특정 언어 실행 코드 |
| Tool call 형식 (`Task(...)`, `Skill(...)`) | 특정 언어 문법에 종속된 코드 |

**좋은 예시:**
```
if score >= 80 AND req_coverage == 100%:
    status = "approve"
else:
    status = "feedback"

for item in items:
    if item.type == "export":
        # Exports 섹션에 추가
```

**피해야 할 예시:**
```python
# Python 특정 문법
items = [x for x in data if x.valid]
result = {"status": "approve"} | extra_fields
```

**예외:** 직접 실행 가능한 스크립트 제공 목적일 경우 특정 언어 코드 허용

## 임시파일 규칙

Agent/Skill 간 결과 전달 시 임시 파일을 사용합니다.

**경로:** `.claude/tmp/{session-id}-{prefix}-{target}.{ext}`

| 요소 | 설명 |
|------|------|
| `{session-id}` | 세션 ID (8자) - 세션별 격리 |
| `{prefix}` | Agent/Skill 고유 접두사 |
| `{target}` | 대상 식별자 (경로의 `/`를 `-`로 변환) |

**파일명 패턴:**

| Component | 파일명 패턴 |
|-----------|-----------|
| drift-validator | `{session-id}-drift-{target}.md` |
| export-validator | `{session-id}-export-{target}.md` |
| decompiler | `{session-id}-decompile-{target}-claude.md`, `{session-id}-decompile-{target}-implements.md` |
| compiler | `{session-id}-compile-{target}.json` |
| test-reviewer | `{session-id}-test-review-{target}.json` |
| audit CLI | `{session-id}-audit-result.json` |
| boundary-resolve | `{session-id}-boundary-{target}.json` |
| code-analyze | `{session-id}-analysis-{target}.json` |
| schema-validate | `{session-id}-validation-{target}.json` |
| spec-agent (state) | `{session-id}-spec-state-{target}.json` |
| spec-reviewer | `{session-id}-review-{target}.json` |
| code-reviewer | `{session-id}-convention-review-{target}.json` |

**예시:** (session-id: a1b2c3d4)
```
.claude/tmp/
├── a1b2c3d4-drift-src-auth.md
├── a1b2c3d4-export-src-auth.md
├── a1b2c3d4-decompile-src-auth-claude.md
├── a1b2c3d4-decompile-src-auth-implements.md
├── a1b2c3d4-compile-src-auth.json
├── a1b2c3d4-test-review-src-auth.json
├── a1b2c3d4-boundary-src-auth.json
├── a1b2c3d4-analysis-src-auth.json
├── a1b2c3d4-audit-result.json
├── a1b2c3d4-spec-state-src-auth.json
├── a1b2c3d4-review-src-auth.json
└── a1b2c3d4-convention-review-src-auth.json
```

**정리:** 세션 종료 시 해당 session-id 접두사의 파일들은 자동 정리됩니다.
