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
    ├── [Planning Section] - /impl이 업데이트
    │   ├── Dependencies Direction
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
| `/impl` | 생성/업데이트 | Planning Section 업데이트 |
| `/compile` | 읽기 전용 | Implementation Section 업데이트 |
| `/decompile` | 생성 (전체) | 생성 (전체 - Planning + Implementation) |

**Exports = 인터페이스 카탈로그, Domain Context = 맥락 카탈로그, IMPLEMENTS.md = 구현 명세**

### Convention Sections

프로젝트/모듈 수준 컨벤션을 CLAUDE.md 내 섹션으로 관리합니다:

- **`## Project Convention`**: project_root CLAUDE.md에 배치. 아키텍처/모듈 구조 규칙.
  - 필수 서브섹션: `### Project Structure`, `### Module Boundaries`, `### Naming Conventions`
  - module_root CLAUDE.md에도 optional override 가능

- **`## Code Convention`**: module_root CLAUDE.md에 배치. 소스코드 수준 규칙.
  - 필수 서브섹션: `### Language & Runtime`, `### Code Style`, `### Naming Rules`
  - 싱글 모듈인 경우 project_root CLAUDE.md에 함께 배치

compiler agent의 REFACTOR 단계에서 자동 참조됩니다. 없으면 project root CLAUDE.md 일반 내용을 fallback으로 사용합니다.

**컨벤션 우선순위** (module_root != project_root인 경우):
1. module_root CLAUDE.md `## Code Convention`
2. module_root CLAUDE.md `## Project Convention` (override)
3. project_root CLAUDE.md `## Code Convention` (default)
4. project_root CLAUDE.md `## Project Convention`
5. project_root CLAUDE.md 일반 내용 (최종 fallback)

## Architecture

### /impl (요구사항 → CLAUDE.md + IMPLEMENTS.md)

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
│    → CLAUDE.md + IMPLEMENTS.md 작성         │
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
│ 6. CLAUDE.md 생성 (WHAT)                    │
│ 7. IMPLEMENTS.md Planning Section 생성 (HOW)│
│ 8. Skill("schema-validate") → 검증 (1회)    │
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
│ Bash(claude-md-core resolve-boundary)       │
│ Bash(claude-md-core analyze-code)           │
│ AskUserQuestion → 불명확한 부분 질문        │
│ CLAUDE.md 생성 (WHAT)                       │
│ IMPLEMENTS.md 생성 (HOW - 전체 섹션)        │
│ Bash(claude-md-core validate-schema)        │
└─────────────────────────────────────────────┘
```

### /compile (CLAUDE.md + IMPLEMENTS.md → 소스코드)

```
User: /compile [--all]
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point)                 │
│                                             │
│ 0. --all 분기                               │
│    ├─ YES → 모든 CLAUDE.md 검색             │
│    └─ NO  → Bash(diff-compile-targets)      │
│             변경 감지                        │
│             판별 조건: staged, modified,     │
│             untracked, spec-newer,           │
│             no-source-code                   │
│             targets = 0 → 종료              │
│ 1. 대상 CLAUDE.md + IMPLEMENTS.md 필터      │
│ 2. IMPLEMENTS.md 없으면 자동 생성           │
│ 3. 언어 자동 감지                           │
│ 4. 의존성 그래프 기반 실행 (leaf-first)     │
│    같은 depth 독립 모듈은 병렬,             │
│    의존 관계는 순차 처리                    │
│    Task(compiler) 호출                      │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ compiler AGENT (TDD Workflow)               │
│                                             │
│ CLAUDE.md 읽기 (WHAT)                       │
│ IMPLEMENTS.md Planning Section 읽기 (HOW)   │
│ Skill("claude-md-parse") → JSON 변환        │
│ [RED] 테스트 생성                           │
│ [GREEN] 구현 생성 (최대 3회 재시도)         │
│ [REFACTOR] 프로젝트 컨벤션 적용             │
│ IMPLEMENTS.md Implementation Section 업데이트│
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
│   Task(validator)                            │
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
| `impl` | 요구사항 분석 및 CLAUDE.md + IMPLEMENTS.md 생성 |
| `dep-explorer` | 요구사항 의존성 탐색 (internal + external) |
| `decompiler` | 소스코드에서 CLAUDE.md 추출 |
| `compiler` | CLAUDE.md에서 소스코드 생성 (TDD) |
| `validator` | CLAUDE.md-코드 일치 검증 및 Export 커버리지 |

## Commands

| Command | 역할 |
|---------|------|
| `/project-setup` | CLAUDE.md에 Convention 섹션 생성 |
| `/convention-update` | CLAUDE.md Convention 섹션 업데이트 |

## Skills

| Skill | 타입 | 역할 |
|-------|------|------|
| `/impl` | Entry Point | 요구사항 → CLAUDE.md |
| `/decompile` | Entry Point | 소스코드 → CLAUDE.md |
| `/compile` | Entry Point | CLAUDE.md → 소스코드 |
| `/validate` | Entry Point | 문서-코드 일치 검증 |
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `boundary-resolve` | Internal | 바운더리 결정 |
| `code-analyze` | Internal | 코드 분석 |
| `claude-md-parse` | Internal | CLAUDE.md 파싱 |
| `scan-claude-md` | CLI Subcommand (not a plugin skill) | 기존 CLAUDE.md 인덱스 생성 (`Bash`에서 `claude-md-core scan-claude-md`로 직접 호출) |
| `diff-compile-targets` | CLI Subcommand (not a plugin skill) | 변경된 CLAUDE.md 감지 (incremental compile용, `Bash`에서 `claude-md-core diff-compile-targets`로 직접 호출) |
| `schema-validate` | Internal | 스키마 검증 |

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
/impl → CLAUDE.md + IMPLEMENTS.md.PlanningSection
/compile → IMPLEMENTS.md.ImplementationSection
/decompile → CLAUDE.md + IMPLEMENTS.md.* (전체)
```

### INV-5: Convention 섹션 배치 규칙
```
project_root/CLAUDE.md MUST contain ## Project Convention
module_root/CLAUDE.md MUST contain ## Code Convention
module_root/CLAUDE.md MAY contain ## Project Convention (override)
싱글 모듈: project_root == module_root → 같은 CLAUDE.md에 두 섹션 모두 배치
```

## 개발 원칙

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **언어 무관**: 파일 확장자 기반 자동 감지
3. **파일 기반 결과 전달**: Agent 결과는 파일로 저장, 경로만 반환
4. **단순한 재시도**: 스키마 검증 1회, 테스트 재시도 3회
5. **버전 관리**: 변경 시 `.claude-plugin/plugin.json`의 `version` 필드를 반드시 bump (patch: 버그/문서, minor: 기능 변경, major: breaking change)
