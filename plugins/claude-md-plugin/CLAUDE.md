# claude-md-plugin

## Purpose

**CLAUDE.md를 Source of Truth로 사용하여 문서-코드 동기화를 구현하는 플러그인.**

기존 접근법(소스코드 → 문서)을 역전시켜 CLAUDE.md가 명세가 되고, 소스코드가 산출물이 되는 패러다임을 제공합니다.

## Core Philosophy: Compile/Decompile 패러다임

**CLAUDE.md는 소스코드이고, 소스코드는 바이너리다.**

```
┌─────────────────────────────────────────────────────────────┐
│                    전통적 소프트웨어                          │
│                                                             │
│   .h (헤더)  ─── compile ──→  Binary (.exe)                 │
│   Binary (.exe)  ─── decompile ──→  .h                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                          │
│                                                             │
│   CLAUDE.md (WHAT)                                          │
│         │                                                   │
│         └──── /compile ──→  Source Code (구현)              │
│                                                             │
│   Source Code (구현)  ─── /decompile ──→  CLAUDE.md (WHAT)  │
└─────────────────────────────────────────────────────────────┘
```

| 전통적 개념 | claude-md-plugin | 역할 |
|------------|------------------|------|
| .h (헤더) | CLAUDE.md | WHAT - 인터페이스, 스펙 |
| Binary | Source Code (.ts, .py, ...) | 실행물 |
| **compile** | CLAUDE.md → Source Code | `/compile` |
| **decompile** | Source Code → CLAUDE.md | `/decompile` |

**보조 문서:**
- **DEVELOPERS.md** (WHY) — 파일관계, 결정근거, 운영 맥락. CLAUDE.md와 1:1 매핑 (INV-3)
- **compile-context** — /impl → /compile 핸드오프용 세션 한정 임시 파일

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

### DEVELOPERS.md = 맥락 문서

**DEVELOPERS.md는 CLAUDE.md와 1:1로 매핑되는 "왜(WHY)" 문서입니다.**

**핵심 원칙:** 현재 상태만 기록, 히스토리는 git에 의존.

```
auth/
├── CLAUDE.md       ← WHAT (스펙)
│   ├── Exports: validateToken(token: string): Claims
│   └── Domain Context: 토큰 만료 7일 (PCI-DSS)
│
└── DEVELOPERS.md   ← WHY (맥락)
    ├── ## File Map           ← 파일별 역할 및 의존관계 (필수, None 불가)
    ├── ## Data Structures    ← 내부 자료구조 관계 (None 허용)
    ├── ## Decision Log       ← ADR 스타일: 맥락/결정/근거 (None 허용)
    └── ## Operations         ← Gotchas/배포/모니터링 (None 허용)
```

### compile-context = 세션 한정 구현 계획

**compile-context는 /impl → /compile 핸드오프용 세션 임시 파일입니다.**

경로: `.claude/tmp/compile-context-{dir-hash}.md`

| 명령어 | CLAUDE.md | compile-context |
|--------|-----------|-----------------|
| `/impl` | 생성/업데이트 | 생성 (세션 한정) |
| `/compile` | 읽기 전용 | 읽기 전용 (optional) |
| `/decompile` | 생성 (전체) | N/A |

**Exports = 인터페이스 카탈로그, Domain Context = 맥락 카탈로그**

### Convention Sections

프로젝트/모듈 수준 컨벤션을 CLAUDE.md 내 섹션으로 관리합니다:

- **`## Project Convention`**: project_root CLAUDE.md에 배치. 아키텍처/모듈 구조 규칙.
  - 필수 서브섹션: `### Project Structure`, `### Module Boundaries`, `### Naming Conventions`
  - module_root CLAUDE.md에도 optional override 가능

- **`## Code Convention`**: project_root CLAUDE.md에 canonical source로 배치. 소스코드 수준 규칙.
  - 필수 서브섹션: `### Language & Runtime`, `### Coding Rules`, `### Naming Rules`
  - module_root CLAUDE.md에는 project_root와 **다른** 내용만 override로 작성 (없으면 상속)
  - 싱글 모듈인 경우 project_root CLAUDE.md에 함께 배치

**DRY 원칙**: Claude Code는 CLAUDE.md를 계층적으로 로드하므로, project_root Convention은
하위 모듈에서 자동 참조됩니다. module_root에는 project_root와 다른 내용만 작성합니다.

compiler agent의 REFACTOR 단계에서 자동 참조됩니다. 없으면 project root CLAUDE.md 일반 내용을 fallback으로 사용합니다.

**컨벤션 우선순위** (module_root != project_root인 경우):
1. module_root CLAUDE.md `## Code Convention`
2. module_root CLAUDE.md `## Project Convention` (override)
3. project_root CLAUDE.md `## Code Convention` (default)
4. project_root CLAUDE.md `## Project Convention`
5. project_root CLAUDE.md 일반 내용 (최종 fallback)

## Architecture

### /impl (요구사항 → CLAUDE.md)

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
│    → CLAUDE.md 작성 + compile-context 생성  │
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
│ 6. CLAUDE.md 생성 (WHAT)                    │
│ 7. compile-context 생성 (세션 한정 HOW)     │
│ 8. Bash(claude-md-core validate-schema) → 검증│
└────────────────────┬────────────────────────┘
                     │
                     ▼ (optional)
┌─────────────────────────────────────────────┐
│ impl-reviewer AGENT (optional review)       │
│                                             │
│ Phase 2-5: D1~D3 차원 분석                  │
│ Phase 6: 점수 산출                          │
│ Phase 7: 대화형 수정 제안 + Edit 적용       │
└─────────────────────────────────────────────┘
```

### /decompile (소스코드 → CLAUDE.md)

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

### /compile (CLAUDE.md → 소스코드)

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
│             targets = 0 → 종료              │
│ 1. 대상 CLAUDE.md 필터                      │
│ 2. compile-context 존재 확인 (optional)     │
│ 3. 언어 자동 감지                           │
│ 4. 의존성 그래프 기반 실행 (leaf-first)     │
│    같은 depth 독립 모듈은 병렬,             │
│    의존 관계는 순차 처리                    │
│    Task(test-designer) → Task(compiler)     │
│    실패 시 피드백 루프 (최대 1회)           │
└────────────────────┬────────────────────────┘
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
┌──────────────────┐  ┌──────────────────────┐
│ test-designer    │  │ compiler AGENT       │
│ AGENT (RED)      │  │ (GREEN + REFACTOR)   │
│                  │  │                      │
│ CLAUDE.md →      │  │ 테스트 Read (R/O)    │
│ Export Tests     │→│ 구현 생성 (3회 재시도)│
│ Behavior Tests   │  │ Convention 적용      │
│ Mock 생성        │  │                      │
└──────────────────┘  └──────────────────────┘
```

### /impl-review (CLAUDE.md 품질 리뷰)

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
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ impl-reviewer AGENT                         │
│                                             │
│ Phase 2-4: D1~D3 차원 분석                  │
│ Phase 5: 점수 산출 & 등급 판정              │
│ Phase 6: 대화형 수정 제안 + Edit 적용       │
└─────────────────────────────────────────────┘
```

### /validate (문서-코드 일치 검증 + 자동 수정)

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
│ 5. Task(issue-fixer) 배치 병렬 → 수정       │
│ 6. 통합 보고서 생성                         │
└────────────────────┬────────────────────────┘
          ┌──────────┼──────────┐
          ▼          ▼          ▼
┌──────────────┐ ┌────────────────┐ ┌─────────────┐
│ validator    │ │ issue-verifier │ │ issue-fixer  │
│ (drift 검증) │ │ (이슈 재검증)  │ │ (CLAUDE.md   │
│              │ │ CONFIRMED/     │ │  수정)       │
│              │ │ FALSE_POSITIVE │ │              │
└──────────────┘ └────────────────┘ └─────────────┘
```

### /bugfix (소스코드 버그 → 3계층 추적 → 수정)

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
│ 5. Task(debugger) → 진단 + 수정            │
│ 6.5. git diff → 수정사항 Diff 표시         │
│ 7. Skill("claude-md-plugin:compile") → 소스코드 재생성 │
│ 8. 검증 (원본 테스트 재실행)                 │
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
│ Phase 7: Fix 제안 + 사용자 승인 + Edit      │
└────────────────────┬────────────────────────┘
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
   ┌────────┐  ┌────────┐  ┌────────┐
   │ L1 분석 │  │ L2 분석 │  │ L3 분석 │
   │ (spec)  │  │(context)│  │ (code)  │
   └────────┘  └────────┘  └────────┘
   debug-layer-analyzer (context 격리)
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

| Agent | 역할 |
|-------|------|
| `impl` | 요구사항 분석 및 CLAUDE.md 생성 + compile-context 생성 |
| `dep-explorer` | 요구사항 의존성 탐색 (internal + external) |
| `decompiler` | 소스코드에서 CLAUDE.md 추출 |
| `test-designer` | CLAUDE.md Exports/Behaviors → 불변 테스트 생성 (RED phase) |
| `compiler` | test-designer 테스트 기반 소스코드 생성 (GREEN + REFACTOR) |
| `debug-layer-analyzer` | 단일 계층(L1/L2/L3) 진단 분석 (debugger의 sub-agent) |
| `debugger` | 소스코드 런타임 버그 → 3계층 추적 → 수정 (orchestrator) |
| `impl-reviewer` | CLAUDE.md 품질 리뷰 및 요구사항 커버리지 검증 |
| `validator` | CLAUDE.md-코드 일치 검증 및 Export 커버리지 |
| `issue-verifier` | 검증 이슈 재검증 (false positive 필터링) |
| `issue-fixer` | 확인된 이슈 기반 CLAUDE.md 자동 수정 |

## Commands

| Command | 역할 |
|---------|------|
| `/dev` | 자연어 요청 분류 → 스킬 라우팅 |
| `/project-setup` | CLAUDE.md에 Convention 섹션 생성 |
| `/convention-update` | CLAUDE.md Convention 섹션 업데이트 |

> **Routing**: `/dev` 스킬로 자연어 요청을 적절한 skill에 라우팅합니다. SessionStart hook은 철학 프레이밍만 주입합니다.

## Skills

| Skill | 타입 | 역할 |
|-------|------|------|
| `/impl` | Entry Point | 요구사항 → CLAUDE.md |
| `/decompile` | Entry Point | 소스코드 → CLAUDE.md |
| `/compile` | Entry Point | CLAUDE.md → 소스코드 |
| `/validate` | Entry Point | 문서-코드 일치 검증 |
| `/bugfix` | Entry Point | 소스코드 런타임 버그 → 3계층 추적 → 수정 |
| `/impl-review` | Entry Point | CLAUDE.md 품질 리뷰 |
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `scan-claude-md` | CLI Subcommand (not a plugin skill) | 기존 CLAUDE.md 인덱스 생성 (`Bash`에서 `claude-md-core scan-claude-md`로 직접 호출) |
| `diff-compile-targets` | CLI Subcommand (not a plugin skill) | 변경된 CLAUDE.md 감지 (incremental compile용, `Bash`에서 `claude-md-core diff-compile-targets`로 직접 호출) |
| `boundary-resolve` | CLI Subcommand (not a plugin skill) | 바운더리 결정 (`Bash`에서 `claude-md-core resolve-boundary`로 직접 호출) |
| `code-analyze` | CLI Subcommand (not a plugin skill) | 코드 분석 (`Bash`에서 `claude-md-core analyze-code`로 직접 호출) |
| `claude-md-parse` | CLI Subcommand (not a plugin skill) | CLAUDE.md 파싱 (`Bash`에서 `claude-md-core parse-claude-md`로 직접 호출) |
| `schema-validate` | CLI Subcommand (not a plugin skill) | 스키마 검증 (`Bash`에서 `claude-md-core validate-schema`로 직접 호출) |
| `format-exports` | CLI Subcommand (not a plugin skill) | analyze-code JSON → deterministic Exports 마크다운 생성 (`Bash`에서 `claude-md-core format-exports`로 직접 호출) |
| `validate-convention` | CLI Subcommand (not a plugin skill) | Convention 섹션 검증 (`Bash`에서 `claude-md-core validate-convention`으로 직접 호출) |
| `fix-schema` | CLI Subcommand (not a plugin skill) | 누락된 allow-none 섹션 자동 추가 (`Bash`에서 `claude-md-core fix-schema`로 직접 호출) |
| `index-project` | CLI Subcommand (not a plugin skill) | 프로젝트 전체 인덱싱: tree-parse + code analysis (`Bash`에서 `claude-md-core index-project`로 직접 호출) |

## 불변식

### INV-1: 트리 구조 의존성
```
node.dependencies ⊆ node.children
```

### INV-2: Self-contained 바운더리
```
validate(node) = validate(node.claude_md, node.direct_files)
```

### INV-3: CLAUDE.md ↔ DEVELOPERS.md 쌍 (활성)
```
∀ CLAUDE.md ∃ DEVELOPERS.md (1:1 mapping)
path(DEVELOPERS.md) = path(CLAUDE.md).replace('CLAUDE.md', 'DEVELOPERS.md')
--strict 모드에서 DEVELOPERS.md 부재를 에러로 보고
```

### INV-4: 업데이트 책임
```
/impl → CLAUDE.md + DEVELOPERS.md + compile-context (세션 한정)
/compile → Source Code (CLAUDE.md + compile-context 읽기 전용)
/decompile → CLAUDE.md + DEVELOPERS.md
/bugfix → CLAUDE.md (L1 fix) → /compile 자동 실행 → Source Code 재생성 → 원본 테스트 검증
/impl-review → CLAUDE.md (사용자 승인 후 fix patch)
/validate → CLAUDE.md + DEVELOPERS.md (drift fix, confirmed issues only via issue-fixer)
```

### INV-5: Convention 섹션 배치 규칙
```
project_root/CLAUDE.md MUST contain ## Project Convention
project_root/CLAUDE.md MUST contain ## Code Convention (canonical source)
module_root/CLAUDE.md MAY contain ## Code Convention (override; 없으면 project_root에서 상속)
module_root/CLAUDE.md MAY contain ## Project Convention (override; 없으면 project_root에서 상속)
싱글 모듈: project_root == module_root → 같은 CLAUDE.md에 두 섹션 모두 배치
```

### INV-EXPORT: Exports 불변식
```
CLAUDE.md Exports 시그니처 = 불변 (정답)
test-designer가 생성한 테스트 구조 = 불변
compiler(GREEN)는 테스트를 수정할 수 없음
Export Interface Tests 실패 → 구현을 수정 (테스트 변경 금지)
```

## 개발 원칙

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **언어 무관**: 파일 확장자 기반 자동 감지
3. **파일 기반 결과 전달**: Agent 결과는 파일로 저장, 경로만 반환
4. **단순한 재시도**: 스키마 검증 1회, 테스트 재시도 3회
5. **버전 관리**: 변경 시 `.claude-plugin/plugin.json`의 `version` 필드를 반드시 bump (patch: 버그/문서, minor: 기능 변경, major: breaking change)
