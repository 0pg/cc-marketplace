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

## Architecture

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
│ Skill("draft-generate") → CLAUDE.md 생성    │
│ Skill("schema-validate") → 스키마 검증      │
└─────────────────────────────────────────────┘
```

### /compile (CLAUDE.md → 소스코드)

기본 동작은 **incremental** (변경분만 처리), `--all` 옵션으로 전체 처리.

```
User: /compile [--all]
        │
        ├─ --all? ─ Yes ─→ 모든 CLAUDE.md 검색
        │                    │
        └─ No ─→ Skill("diff-analyze")
                   │
                   ├─ 변경 없음 → 조기 종료
                   └─ 변경 있음 → 변경된 파일만
                                   │
        ←───────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│ compile SKILL (Entry Point)                 │
│                                             │
│ 1. Skill("diff-analyze") → 변경 감지        │
│    (--all 시 전체 CLAUDE.md 검색)           │
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
│ [RED] Skill("signature-convert") → 테스트   │
│ [GREEN] 구현 생성 (최대 5회 재시도)          │
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
│   Task(reproducibility-validator)           │
└─────────────────────────────────────────────┘
```

### 설계 원칙

| 컴포넌트 | 역할 | 오케스트레이션 |
|----------|------|---------------|
| **Entry Point Skill** | 사용자 진입점 | 간단 (파일 검색, 반복, Agent 호출) |
| **Internal Skill** | 단일 기능 (SRP) | 없음, Stateless |
| **Agent** | 비즈니스 로직 | 복잡 (N개 Skill, 재시도, 상태) |

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
