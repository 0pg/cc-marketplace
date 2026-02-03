# claude-md-plugin

## Purpose

**CLAUDE.md를 Source of Truth로 사용하여 문서-코드 동기화를 구현하는 플러그인.**

기존 접근법(소스코드 → 문서)을 역전시켜 CLAUDE.md가 명세가 되고, 소스코드가 산출물이 되는 패러다임을 제공합니다.

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

## Architecture

### Agent-Skill 아키텍처

#### /extract 워크플로우 (소스코드 → CLAUDE.md)

```
User: /extract
        │
        ▼
┌─────────────────────────────────────────────┐
│ extract SKILL (Entry Point)                 │
│ ─────────────────────────────────────────── │
│ 간단한 오케스트레이션                        │
│                                             │
│ 1. Skill("tree-parse") → 대상 목록          │
│ 2. For each directory (leaf-first):         │
│    Task(extractor) 호출                     │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ extractor AGENT (복잡한 오케스트레이터)      │
│ ─────────────────────────────────────────── │
│ 5 Skill 호출, 조건문, 재시도 로직            │
│                                             │
│ ┌─ Skill("boundary-resolve") ── Internal ┐  │
│ │ 바운더리 분석                           │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("code-analyze") ─── Internal ───┐  │
│ │ 코드 분석 (exports, deps, behaviors)    │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ AskUserQuestion ──────────────────────┐  │
│ │ 불명확한 부분 질문                      │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("draft-generate") ── Internal ──┐  │
│ │ CLAUDE.md 초안 생성                     │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("schema-validate") ── Internal ─┐  │
│ │ 스키마 검증 (실패시 재시도)             │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

#### /generate 워크플로우 (CLAUDE.md → 소스코드)

```
User: /generate
        │
        ▼
┌─────────────────────────────────────────────┐
│ generate SKILL (Entry Point)                │
│ ─────────────────────────────────────────── │
│ 간단한 오케스트레이션                        │
│                                             │
│ 1. CLAUDE.md 파일 검색                      │
│ 2. 언어 자동 감지                           │
│ 3. For each CLAUDE.md:                      │
│    Task(generator) 호출                     │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ generator AGENT (복잡한 오케스트레이터)      │
│ ─────────────────────────────────────────── │
│ 4+ Skill 호출, 재시도 로직, 상태 관리        │
│                                             │
│ ┌─ Skill("claude-md-parse") ── Internal ─┐  │
│ │ CLAUDE.md → ClaudeMdSpec JSON          │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ TDD Workflow (내부 자동) ─────────────┐  │
│ │                                        │  │
│ │ [RED] behaviors → 테스트 생성 (실패)   │  │
│ │       └─ Skill("signature-convert")    │  │
│ │              └── Internal ────────     │  │
│ │                   │                    │  │
│ │                   ▼                    │  │
│ │ [GREEN] 구현 생성 + 테스트 통과        │  │
│ │         └─ 최대 5회 재시도             │  │
│ │                   │                    │  │
│ │                   ▼                    │  │
│ │ [REFACTOR] 프로젝트 컨벤션 적용        │  │
│ │                                        │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ 파일 충돌 처리 ───────────────────────┐  │
│ │ skip (기본) 또는 overwrite 모드        │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

#### /incremental-generate 워크플로우 (변경된 CLAUDE.md만 처리)

```
User: /incremental-generate
          │
          ▼
┌──────────────────────────────────────────┐
│ incremental-generate SKILL (Entry Point) │
│ ──────────────────────────────────────── │
│ 간단한 오케스트레이션                     │
│                                          │
│ 1. Skill("diff-analyze") 호출            │
│ 2. 변경된 CLAUDE.md 목록 획득            │
│ 3. For each: Task(generator) 호출        │
│ 4. 결과 수집 및 보고                      │
└────────────────────┬─────────────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│ diff-analyze    │    │ generator AGENT │
│ SKILL (Internal)│    │ (복잡한         │
│                 │    │  오케스트레이터) │
│ git merge-base  │    │                 │
│ + git diff      │    │ TDD workflow    │
└─────────────────┘    └─────────────────┘
```

#### /validate 워크플로우 (문서-코드 일치 검증)

```
User: /validate
        │
        ▼
┌─────────────────────────────────────────────┐
│ validate SKILL (Entry Point)                │
│ ─────────────────────────────────────────── │
│ 간단한 오케스트레이션                        │
│                                             │
│ 1. Glob("**/CLAUDE.md") → 대상 목록         │
│ 2. mkdir .claude/validate-results           │
│ 3. For each CLAUDE.md (병렬):               │
│    Task(drift-validator) 호출               │
│    Task(reproducibility-validator) 호출     │
└────────────────────┬────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
        ▼                         ▼
┌───────────────────┐   ┌───────────────────┐
│ drift-validator   │   │ reproducibility-  │
│ AGENT             │   │ validator AGENT   │
│ (복잡한           │   │ (복잡한           │
│  오케스트레이터)   │   │  오케스트레이터)   │
│                   │   │                   │
│ Structure drift   │   │ Phase 1: 예측    │
│ Exports drift     │   │ (코드 읽지 않음)  │
│ Dependencies drift│   │                   │
│ Behavior drift    │   │ Phase 2: 검증    │
│                   │   │ (실제 코드 비교)  │
│ → drift-*.md 저장 │   │ → repro-*.md 저장│
└─────────┬─────────┘   └─────────┬─────────┘
          │                       │
          └───────────┬───────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│ validate SKILL (Entry Point - 결과 수집)    │
│                                             │
│ 1. Read 결과 파일들                          │
│ 2. 통합 보고서 생성                          │
│ 3. rm -rf .claude/validate-results          │
└─────────────────────────────────────────────┘
```

### 설계 원칙

#### Skill 구분

**Entry Point Skill** (사용자 진입점)
- 사용자가 직접 호출 (`/extract`, `/generate`, `/validate`, `/incremental-generate`)
- description에 사용자 친화적 설명
- 간단한 오케스트레이션 허용:
  - 대상 파일/디렉토리 검색
  - 반복 처리 (For each)
  - Agent 호출 (`Task(agent-name)`)

**Internal Skill** (내부 전용)
- Agent에서만 호출 `Skill("claude-md-plugin:skill-name")`
- 단일 기능 (SRP), Stateless, 재사용 가능
- description에 `(internal)` 표시하여 자동완성에서 숨김

#### Agent (비즈니스 오케스트레이터)
- N개의 Skill을 `Skill("claude-md-plugin:skill-name")` 형태로 호출
- 복잡한 오케스트레이션 담당:
  - 4개 이상 Skill 호출
  - 조건문, 반복문
  - 재시도 로직 (최대 N회)
  - 상태 관리
- 워크플로우 관리, 에러 처리
- 사용자에게 비즈니스 가치 제공

## Structure

```
plugins/claude-md-plugin/
├── .claude-plugin/
│   └── plugin.json          # 플러그인 매니페스트
├── CLAUDE.md                # 이 파일
├── README.md                # 사용자 문서
├── core/                    # Rust CLI (Core Engine)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── tree_parser.rs
│   │   ├── boundary_resolver.rs
│   │   ├── schema_validator.rs
│   │   ├── claude_md_parser.rs    # CLAUDE.md 파싱
│   │   ├── signature_converter.rs # 시그니처 변환
│   │   └── main.rs
│   └── tests/
│       └── features/        # Gherkin 테스트
│           ├── tree_parser.feature
│           ├── boundary_resolver.feature
│           ├── schema_validator.feature
│           ├── claude_md_parser.feature    # 파서 테스트
│           ├── signature_converter.feature # 변환기 테스트
│           └── code_generator.feature      # 생성기 테스트
├── skills/
│   ├── extract/
│   │   └── SKILL.md         # /extract (사용자 진입점)
│   ├── generate/
│   │   └── SKILL.md         # /generate (사용자 진입점)
│   ├── incremental-generate/
│   │   └── SKILL.md         # /incremental-generate (사용자 진입점)
│   ├── validate/
│   │   └── SKILL.md         # /validate (사용자 진입점)
│   ├── diff-analyze/
│   │   └── SKILL.md         # (internal) Git diff 기반 변경 감지
│   ├── tree-parse/
│   │   └── SKILL.md         # (internal) 트리 파싱
│   ├── boundary-resolve/
│   │   └── SKILL.md         # (internal) 바운더리 분석
│   ├── code-analyze/
│   │   └── SKILL.md         # (internal) 코드 분석
│   ├── draft-generate/
│   │   └── SKILL.md         # (internal) CLAUDE.md 생성
│   ├── schema-validate/
│   │   └── SKILL.md         # (internal) 스키마 검증
│   ├── claude-md-parse/
│   │   └── SKILL.md         # (internal) CLAUDE.md 파싱
│   └── signature-convert/
│       └── SKILL.md         # (internal) 시그니처 변환
├── agents/
│   ├── extractor.md         # Skill 조합으로 단일 디렉토리 처리
│   ├── generator.md         # CLAUDE.md → 소스코드 생성
│   ├── drift-validator.md   # 코드-문서 일치 검증
│   └── reproducibility-validator.md  # 재현성 검증
└── templates/
    └── claude-md-schema.md  # CLAUDE.md 스키마 정의
```

## Core Engine 컴포넌트

### tree_parser
| 기능 | 설명 |
|------|------|
| 디렉토리 스캔 | 재귀적으로 프로젝트 구조 탐색 |
| 소스 파일 탐지 | 확장자 기반 (.ts, .py, .go, .rs, .java, .kt 등) |
| CLAUDE.md 필요 판정 | 1+ 소스파일 OR 2+ 하위디렉토리 |
| 제외 처리 | 빌드 디렉토리 (node_modules, target, dist 등) |

### boundary_resolver
| 기능 | 설명 |
|------|------|
| 바운더리 결정 | 직접 파일 목록, 하위 디렉토리 목록 |
| 참조 검증 | Parent/Sibling 참조 위반 탐지 |

### schema_validator
| 검증 항목 | 규칙 |
|----------|------|
| 필수 섹션 | Purpose, Exports, Behavior 필수 |
| Exports 형식 | `Name(params) ReturnType` 패턴 |
| Behavior 형식 | `input → output` 패턴 |

### claude_md_parser
| 기능 | 설명 |
|------|------|
| CLAUDE.md 파싱 | Purpose, Exports, Dependencies, Behaviors, Contracts, Protocol 섹션 추출 |
| ClaudeMdSpec 생성 | 구조화된 JSON 스펙 출력 |
| 다중 언어 지원 | TypeScript, Python, Go, Rust, Java, Kotlin |

### signature_converter
| 기능 | 설명 |
|------|------|
| 시그니처 변환 | 범용 시그니처 → 대상 언어 변환 |
| 타입 매핑 | string→str(Python), string→String(Go) 등 |
| 네이밍 규칙 | camelCase→snake_case(Python), PascalCase(Go) 등 |
| Promise 처리 | Promise<T>→async/await(Python), (T, error)(Go) 등 |

## CLI Interface

```bash
# 트리 파싱
claude-md-core parse-tree --root . --output tree.json

# 바운더리 결정
claude-md-core resolve-boundary --path src/auth --output boundary.json

# 스키마 검증
claude-md-core validate-schema --file CLAUDE.md --output validation.json

# CLAUDE.md 파싱 (NEW)
claude-md-core parse-claude-md --file CLAUDE.md --output spec.json

# 시그니처 변환 (NEW)
claude-md-core convert-signature --signature "<시그니처>" --target-lang <언어>
# 입력 시그니처를 대상 언어의 관용적 표현으로 변환
```

## Workflow

### /extract 실행 시 (소스코드 → CLAUDE.md)
1. extract Skill이 `Skill("tree-parse")` 호출
2. tree.json에서 CLAUDE.md 필요한 디렉토리 추출 (leaf-first 정렬)
3. 각 디렉토리에 `Task(extractor)` 실행 (순차)
4. extractor Agent가 내부 Skill 조합으로 CLAUDE.md 생성
5. 결과 수집 및 보고

### /generate 실행 시 (CLAUDE.md → 소스코드)
1. generate Skill이 프로젝트에서 CLAUDE.md 파일 검색
2. 각 CLAUDE.md가 있는 디렉토리의 언어 자동 감지
   - 기존 소스 파일 확장자 기반 (.ts, .py, .go, .rs, .java, .kt)
   - 감지 불가 시 사용자에게 질문
3. 각 CLAUDE.md에 대해 `Task(generator)` 실행
4. generator Agent가 내부 TDD 워크플로우 수행:
   - [RED] behaviors → 테스트 코드 생성 (실패 확인)
   - [GREEN] 구현 생성 + 테스트 통과 (최대 5회 재시도)
   - [REFACTOR] 프로젝트 컨벤션 적용
5. 파일 충돌 처리 (skip 또는 overwrite)
6. 결과 수집 및 보고

### /incremental-generate 실행 시 (변경된 CLAUDE.md만 처리)
1. incremental-generate Skill이 `Skill("diff-analyze")` 호출
2. Git merge-base 기준으로 변경된 CLAUDE.md 파일 목록 획득
3. 변경 없으면 조기 종료, 있으면 변경 내역 보고
4. 각 변경된 CLAUDE.md에 대해 `Task(generator)` 실행 (기존 generator Agent 재사용)
5. 결과 수집 및 보고 (처리 수, 건너뜀 수, 테스트 결과)

### /validate 실행 시 (문서-코드 일치 검증)
1. validate Skill이 대상 경로에서 CLAUDE.md 파일 검색
2. `.claude/validate-results/` 디렉토리 생성
3. 각 CLAUDE.md에 대해 **병렬로** Task 실행:
   - `drift-validator`: Structure, Exports, Dependencies, Behavior drift 검증
   - `reproducibility-validator`: CLAUDE.md만으로 코드 재현 가능 여부 검증
4. 결과 파일들 수집 및 통합 보고서 생성
5. 임시 파일 정리 (`.claude/validate-results/` 삭제)

## 불변식

### INV-1: 트리 구조 의존성
```
node.dependencies ⊆ node.children
```

### INV-2: Self-contained 바운더리
```
validate(node) = validate(node.claude_md, node.direct_files)
```

## 개발 원칙

1. **ATDD**: Gherkin feature 먼저 작성, 이후 구현
2. **언어 무관**: Core Engine은 모든 프로그래밍 언어 지원
3. **파일 기반 결과 전달**: Agent 결과는 파일로 저장, 경로만 반환

## Rust Core Engine 코딩 규칙

### 금지 사항
1. **unsafe 사용 금지**: 모든 코드는 safe Rust로 작성
2. **panic 유발 코드 금지**:
   - `.unwrap()` 대신 `?` 연산자 또는 `.unwrap_or()`, `.unwrap_or_else()` 사용
   - `.expect()` 대신 적절한 에러 처리
   - 배열 인덱싱 대신 `.get()` 사용

### 필수 사항
3. **상세한 에러 메시지**: 실패 시 사유를 명확하게 전달
   - 파일 경로, 실패 원인, 가능한 해결책 포함
   - `thiserror` 크레이트로 구조화된 에러 타입 사용

## 언어 지원

**프로젝트에서 사용하는 언어와 테스트 프레임워크를 자동 감지합니다.**

- 언어 감지: 파일 확장자 기반
- 테스트 프레임워크 감지: 프로젝트 설정 파일 분석 (package.json, pyproject.toml, Cargo.toml, build.gradle 등)
- 네이밍 규칙: 프로젝트 root CLAUDE.md의 코딩 컨벤션을 따름

## 향후 계획 (현재 범위 아님)

- **Round-trip 테스트**: Extract → Generate → Compare
