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

```
User: /extract
        │
        ▼
┌─────────────────────────────────────────────┐
│ extract SKILL (사용자 진입점)                │
│                                             │
│ 1. Skill("tree-parse") → 대상 목록          │
│ 2. For each directory (leaf-first):         │
│    Task(extractor) 생성                     │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ extractor AGENT (디렉토리별)                 │
│                                             │
│ ┌─ Skill("boundary-resolve") ────────────┐  │
│ │ 바운더리 분석                           │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("code-analyze") ────────────────┐  │
│ │ 코드 분석 (exports, deps, behaviors)    │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ AskUserQuestion ──────────────────────┐  │
│ │ 불명확한 부분 질문                      │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("draft-generate") ──────────────┐  │
│ │ CLAUDE.md 초안 생성                     │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("schema-validate") ─────────────┐  │
│ │ 스키마 검증 (실패시 재시도)             │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### 설계 원칙

#### Skill (도메인 컴포넌트)
- **한 가지 일만** 잘 수행 (SRP)
- Stateless, 재사용 가능
- 내부 전용 Skill은 description에 `(internal)` 표시하여 자동완성에서 숨김

#### Agent (비즈니스 오케스트레이터)
- N개의 Skill을 `Skill("claude-md-plugin:skill-name")` 형태로 호출
- 워크플로우 관리, 에러 처리, 재시도 로직 담당
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
│   │   └── main.rs
│   └── tests/
│       └── features/        # Gherkin 테스트
├── skills/
│   ├── extract/
│   │   └── SKILL.md         # /extract (사용자 진입점)
│   ├── tree-parse/
│   │   └── SKILL.md         # (internal) 트리 파싱
│   ├── boundary-resolve/
│   │   └── SKILL.md         # (internal) 바운더리 분석
│   ├── code-analyze/
│   │   └── SKILL.md         # (internal) 코드 분석
│   ├── draft-generate/
│   │   └── SKILL.md         # (internal) CLAUDE.md 생성
│   └── schema-validate/
│       └── SKILL.md         # (internal) 스키마 검증
├── agents/
│   └── extractor.md         # Skill 조합으로 단일 디렉토리 처리
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

## CLI Interface

```bash
# 트리 파싱
claude-md-core parse-tree --root . --output tree.json

# 바운더리 결정
claude-md-core resolve-boundary --path src/auth --output boundary.json

# 스키마 검증
claude-md-core validate-schema --file CLAUDE.md --output validation.json
```

## Workflow

### /extract 실행 시
1. extract Skill이 `Skill("tree-parse")` 호출
2. tree.json에서 CLAUDE.md 필요한 디렉토리 추출 (leaf-first 정렬)
3. 각 디렉토리에 `Task(extractor)` 실행 (순차)
4. extractor Agent가 내부 Skill 조합으로 CLAUDE.md 생성
5. 결과 수집 및 보고

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

## 향후 계획 (현재 범위 아님)

- **Validator Agent**: 80% 재현율 검증
- **Generator Agent**: CLAUDE.md → 소스코드 생성
