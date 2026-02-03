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
│   Source Code (.c, .java)  ─── compile ──→  Binary (.exe)  │
│   Binary (.exe)  ─── decompile ──→  Source Code            │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    claude-md-plugin                         │
│                                                             │
│   CLAUDE.md (스펙)  ─── /compile ──→  Source Code (구현)    │
│   Source Code (구현)  ─── /decompile ──→  CLAUDE.md (스펙)  │
└─────────────────────────────────────────────────────────────┘
```

| 전통적 개념 | claude-md-plugin | 명령어 |
|------------|------------------|--------|
| Source Code | CLAUDE.md | - |
| Binary | Source Code (.ts, .py, ...) | - |
| **compile** | CLAUDE.md → Source Code | `/compile` |
| **decompile** | Source Code → CLAUDE.md | `/decompile` |

**왜 이 비유인가?**
- **Source Code**는 사람이 읽고 작성하는 것 → CLAUDE.md
- **Binary**는 기계가 실행하는 것 → 실제 소스코드 (런타임이 실행)
- **Compile**은 스펙에서 실행 가능한 형태로 변환 → CLAUDE.md에서 코드 생성
- **Decompile**은 실행 가능한 형태에서 스펙 추출 → 코드에서 CLAUDE.md 추출

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

### Domain Context = 맥락 카탈로그

**Domain Context 섹션은 compile 재현성을 보장하는 맥락 정보입니다.**

이 정보가 없으면 동일한 CLAUDE.md에서 다른 코드가 생성될 수 있습니다.

| 역할 | 설명 |
|------|------|
| **자체 compile 재현** | 해당 CLAUDE.md → 동일한 코드 |
| **의존자 compile 영향** | 이 모듈을 참조하는 다른 모듈의 compile 결정에 필요한 맥락 |

```
auth/CLAUDE.md
├── Exports: validateToken(token: string): Claims  ← 인터페이스
└── Domain Context: 토큰 만료 7일 (PCI-DSS)       ← 맥락

user/CLAUDE.md (auth 의존)
├── compile 시 auth/Exports 참조 → validateToken 호출 방법
└── compile 시 auth/Domain Context 참조 → 세션 갱신 주기 결정
```

**Exports = 인터페이스 카탈로그, Domain Context = 맥락 카탈로그**

## Architecture

### /spec (요구사항 → CLAUDE.md)

```
User: /spec "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL (Entry Point)                    │
│                                             │
│ Task(spec-agent) → 요구사항 분석 및         │
│                    CLAUDE.md 작성           │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ spec-agent AGENT                            │
│                                             │
│ 1. 요구사항 분석                            │
│ 2. AskUserQuestion → 모호한 부분 명확화     │
│ 3. 대상 경로 결정                           │
│ 4. 기존 CLAUDE.md 병합 (필요시)             │
│ 5. CLAUDE.md 생성                           │
│ 6. Skill("schema-validate") → 검증 (1회)    │
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
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ decompiler AGENT                            │
│                                             │
│ Skill("boundary-resolve") → 바운더리 분석   │
│ Skill("code-analyze") → 코드 분석           │
│ AskUserQuestion → 불명확한 부분 질문        │
│ CLAUDE.md 생성 (인라인)                     │
│ Skill("schema-validate") → 검증 (1회)       │
└─────────────────────────────────────────────┘
```

### /compile (CLAUDE.md → 소스코드)

```
User: /compile
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point)                 │
│                                             │
│ 1. 모든 CLAUDE.md 검색                      │
│ 2. 언어 자동 감지                           │
│ 3. For each CLAUDE.md (병렬):               │
│    Task(compiler) 호출                      │
└────────────────────┬────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────┐
│ compiler AGENT (TDD Workflow)               │
│                                             │
│ Skill("claude-md-parse") → JSON 변환        │
│ [RED] 테스트 생성                           │
│ [GREEN] 구현 생성 (최대 3회 재시도)         │
│ [REFACTOR] 프로젝트 컨벤션 적용             │
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
| `spec-agent` | 요구사항 분석 및 CLAUDE.md 생성 |
| `decompiler` | 소스코드에서 CLAUDE.md 추출 |
| `compiler` | CLAUDE.md에서 소스코드 생성 (TDD) |
| `drift-validator` | CLAUDE.md-코드 일치 검증 |
| `export-validator` | Export 존재 검증 |

## Skills

| Skill | 타입 | 역할 |
|-------|------|------|
| `/spec` | Entry Point | 요구사항 → CLAUDE.md |
| `/decompile` | Entry Point | 소스코드 → CLAUDE.md |
| `/compile` | Entry Point | CLAUDE.md → 소스코드 |
| `/validate` | Entry Point | 문서-코드 일치 검증 |
| `tree-parse` | Internal | 디렉토리 구조 분석 |
| `boundary-resolve` | Internal | 바운더리 결정 |
| `code-analyze` | Internal | 코드 분석 |
| `claude-md-parse` | Internal | CLAUDE.md 파싱 |
| `schema-validate` | Internal | 스키마 검증 |

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
2. **언어 무관**: 파일 확장자 기반 자동 감지
3. **파일 기반 결과 전달**: Agent 결과는 파일로 저장, 경로만 반환
4. **단순한 재시도**: 스키마 검증 1회, 테스트 재시도 3회
