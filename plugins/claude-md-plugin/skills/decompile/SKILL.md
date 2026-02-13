---
name: decompile
version: 1.1.0
aliases: [decom]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", or uses "/decompile" or "/decom".
  Analyzes existing source code (binary) and creates CLAUDE.md (source) documentation for each directory.
  Trigger keywords: 디컴파일, 코드에서 문서 추출, 기존 코드 문서화
user_invocable: true
allowed-tools: [Bash, Read, Write, Glob, Task, Skill, AskUserQuestion]
---

# Decompile Skill

## Core Philosophy

**Source Code(바이너리) → CLAUDE.md + IMPLEMENTS.md(소스) = Decompile**

```
Source Code (구현)  ─── /decompile ──→  CLAUDE.md (WHAT) + IMPLEMENTS.md (HOW)
```

전통적 디컴파일러가 바이너리에서 소스코드를 추출하듯,
`/decompile`은 기존 소스코드에서 CLAUDE.md + IMPLEMENTS.md 명세를 추출.

## 듀얼 문서 시스템

| 출력 | 역할 | 내용 |
|------|------|------|
| CLAUDE.md | WHAT (스펙) | Purpose, Exports, Behavior, Contract, Protocol, Domain Context |
| IMPLEMENTS.md Planning | HOW 계획 | Dependencies Direction, Implementation Approach, Technology Choices |
| IMPLEMENTS.md Implementation | HOW 상세 | Algorithm, Key Constants, Error Handling, State Management, Implementation Guide |

## 워크플로우

```
/decompile
    │
    ▼
Skill("tree-parse") → tree.json 파일 경로
    │
    ▼
Bash("jq -r '.needs_claude_md | sort_by(-.depth) | .[] | ...' {tree_file}")
→ "depth path" 한 줄씩 추출 (main context에 tree.json 전체를 Read하지 않음)
    │
    ▼
For each directory (leaf-first 순차):
    1. 자식 CLAUDE.md 목록 수집 (Bash jq)
    2. Task(decompiler) → result block만 반환 (~5줄)
    3. status 확인 → 다음 디렉토리
    │
    ▼
최종 보고
```

**핵심:** 자식 디렉토리 CLAUDE.md가 먼저 생성되어야 부모가 자식의 Purpose를 읽을 수 있음.

**응답 압축 방식:** decompiler agent에게 result block만 반환하도록 지시하여
main context 적재를 최소화. Task prompt도 압축 형식 사용.

**tree-parse 결과 전달:** decompiler agent가 tree.json을 직접 jq로 조회하여
디렉토리 정보를 획득. prompt에는 최소 정보만 포함.

## 대상 요약 표시 후 즉시 실행

tree-parse 후 jq로 디렉토리 수만 확인하여 요약 표시. tree.json 전체를 Read하지 않음.

```bash
# 디렉토리 수 확인
Bash("jq '.needs_claude_md | length' {tree_file}")

# depth-path 목록 추출 (반복문에서 사용)
Bash("jq -r '.needs_claude_md | sort_by(-.depth) | .[] | \"\\(.depth) \\(.path)\"' {tree_file}")
```

```
=== Decompile 시작 ===

대상: 57개 디렉토리
처리 순서: leaf-first

[1/57] src/auth/jwt/ ...
```

**주의:** 사용자에게 확인을 묻지 않고 바로 실행. `/decompile` 호출 자체가 실행 의도임.

## 내부 도구 목록

| 도구 | 역할 | 호출 위치 |
|------|------|----------|
| `tree-parse` Skill | 디렉토리 트리 파싱 | decompile Skill |
| `claude-md-core resolve-boundary` CLI | 바운더리 분석 | decompiler Agent |
| `claude-md-core analyze-code` CLI | 코드 분석 | decompiler Agent |
| `claude-md-core validate-schema` CLI | 스키마 검증 | decompiler Agent |

## 최종 보고 예시

```
=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md + IMPLEMENTS.md
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md
  ✓ src/api/CLAUDE.md + IMPLEMENTS.md

검증 결과: 3/3 통과

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
```

## 결과 전달

decompiler agent가 CLAUDE.md와 IMPLEMENTS.md를 **대상 디렉토리에 직접 Write**.
scratchpad 중간 저장 및 Read+Write 복사 과정 없음 (context 적재 방지).

### 흐름 (Foreground + 압축 응답)

1. `Task(decompiler)` → result block만 반환 (~5줄).
2. result block에서 status 확인.
3. status가 success면 다음 디렉토리 처리. 그 외 경고 후 skip.

### Agent result block 형식

```
---decompiler-result---
status: success
target_dir: {target_dir}
validation: passed | failed_with_warnings
---end-decompiler-result---
```

## DO / DON'T

**DO:**
- Process directories in leaf-first order
- Instruct decompiler agent to return result block only (압축 응답)
- Use jq for tree.json field extraction (NOT Read for full JSON)
- Write results directly to target directory (NOT to scratchpad)

**DON'T:**
- Read tree.json into main context (use jq extraction)
- Overwrite existing CLAUDE.md without user confirmation
- Skip schema validation
- Generate code (decompile only extracts documentation)
- Use Read → Write to copy files (loads content into main context)
- Include file contents in decompiler result block

## 참조 문서

- `references/execution-order.md`: 상세 실행 순서 (참조용, 런타임 로딩 불필요)
- `examples/final-report.md`: 최종 보고 형식 예시

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | 에러 메시지 출력, 실패 반환 |
| tree-parse 실패 | CLI 에러 메시지 전달 |
| jq 명령 실패 | tree.json 경로 확인, 재시도 또는 실패 반환 |
| decompiler 실패 | 해당 디렉토리 스킵, 경고 표시 |
| 스키마 검증 실패 | 경고와 함께 진행 (재시도 없음) |

## Task Prompt 형식 (압축)

decompiler agent에게 전달하는 prompt는 최소 정보만 포함:

```
대상: {path}  tree: {tree_file}
자식 CLAUDE.md: {children_list}
```

Agent가 tree.json에서 직접 jq로 `source_file_count`, `subdir_count` 등을 조회합니다.

## Examples

<example>
<context>
사용자가 기존 코드베이스를 문서화하려고 합니다.
</context>
<user_request>/decompile src/</user_request>
<assistant_response>
디렉토리 구조를 분석합니다...

=== CLAUDE.md + IMPLEMENTS.md 추출 완료 ===

생성된 파일:
  ✓ src/auth/CLAUDE.md + IMPLEMENTS.md
  ✓ src/utils/CLAUDE.md + IMPLEMENTS.md
  ✓ src/CLAUDE.md + IMPLEMENTS.md

검증 결과: 3/3 통과

다음 단계:
  - /validate로 문서-코드 일치 검증 가능
  - /compile로 코드 재생성 가능 (재현성 테스트)
</assistant_response>
</example>
