---
name: smoke-test
description: |
  [DEV ONLY] 플러그인 개발자용 E2E 스모크 테스트.
  배포 대상이 아니며, 플러그인 skills/ 에 포함되지 않습니다.
  실행: 이 파일을 Claude에게 읽히고 워크플로우를 따르도록 요청하세요.
allowed-tools: [Bash, Read, Write, Glob, Grep, Task, Skill, AskUserQuestion]
---

# Smoke Test (Dev Only)

<example>
<context>
사용자가 전체 UC 파이프라인이 정상 동작하는지 확인하려고 합니다.
</context>
<user>/smoke-test</user>
<assistant_response>
Smoke Test Report
=================

Phase 1: /impl  .............. PASS
Phase 2: /compile ............ PASS
Phase 3: /decompile .......... PASS
Phase 4: /validate ........... PASS

Result: 4/4 PASS
</assistant_response>
</example>

전체 UC 파이프라인을 E2E로 실행하여 agent-CLI 체인이 런타임에 정상 동작하는지 검증합니다.

## Triggers

- `/smoke-test`
- `E2E 검증`
- `파이프라인 테스트`

## Arguments

없음. 모든 설정은 자동으로 처리됩니다.

## 워크플로우

### Phase 0: Setup

임시 테스트 프로젝트를 생성합니다.

```
smoke_dir = {project_root}/.claude/tmp/smoke-test-{session-id}/

# 디렉토리 구조 생성
mkdir -p {smoke_dir}/src

# 최소 소스 파일 생성
Write({smoke_dir}/src/calculator.ts):
  export function add(a: number, b: number): number {
    return a + b;
  }

  export function subtract(a: number, b: number): number {
    return a - b;
  }
```

### Phase 1: /impl 검증

impl-agent를 호출하여 CLAUDE.md + IMPLEMENTS.md를 생성합니다.

```
Task(impl-agent, prompt="""
사용자 요구사항:
src/calculator.ts의 add, subtract 함수에 대한 CLAUDE.md를 생성해주세요.
두 함수는 숫자 두 개를 받아 각각 덧셈, 뺄셈 결과를 반환합니다.

프로젝트 루트: {smoke_dir}
대상 경로: {smoke_dir}/src

주의: AskUserQuestion 사용하지 마세요. 제공된 정보만으로 CLAUDE.md + IMPLEMENTS.md를 생성하세요.
스키마 검증도 수행하세요.
""")
```

**검증 체크리스트:**

| 항목 | 검증 방법 |
|------|----------|
| CLAUDE.md 생성 | `Glob("**/CLAUDE.md", path={smoke_dir})` 결과 1개 이상 |
| IMPLEMENTS.md 생성 | `Glob("**/IMPLEMENTS.md", path={smoke_dir})` 결과 1개 이상 |
| Schema 검증 | `claude-md-core validate-schema --file {smoke_dir}/src/CLAUDE.md` 통과 |

### Phase 2: /compile 검증

Phase 1에서 생성된 CLAUDE.md로 compiler를 호출합니다.

```
Task(compiler, prompt="""
CLAUDE.md 경로: {smoke_dir}/src/CLAUDE.md
IMPLEMENTS.md 경로: {smoke_dir}/src/IMPLEMENTS.md
프로젝트 루트: {smoke_dir}
언어: TypeScript
phase: full

CLAUDE.md를 읽고 소스 코드와 테스트를 생성해주세요.
결과는 {smoke_dir}/src/ 디렉토리에 저장합니다.
""")
```

**검증 체크리스트:**

| 항목 | 검증 방법 |
|------|----------|
| 소스 코드 생성 | `Glob("**/*.ts", path={smoke_dir}/src)` 에서 CLAUDE.md/IMPLEMENTS.md 외 파일 1개 이상 |
| 테스트 생성 | `Glob("**/*.test.ts", path={smoke_dir})` 또는 `Glob("**/*.spec.ts", path={smoke_dir})` 결과 1개 이상 |

### Phase 3: /decompile 검증

Phase 2에서 생성된 코드를 기반으로 decompiler를 호출합니다.

먼저 Phase 1에서 생성된 CLAUDE.md와 IMPLEMENTS.md를 백업 후 삭제합니다:

```
# 백업
Read({smoke_dir}/src/CLAUDE.md) → claude_md_backup
Read({smoke_dir}/src/IMPLEMENTS.md) → implements_md_backup

# 삭제 (decompile이 새로 생성하도록)
Bash: rm {smoke_dir}/src/CLAUDE.md {smoke_dir}/src/IMPLEMENTS.md
```

decompiler를 호출합니다:

```
Task(decompiler, prompt="""
대상 디렉토리: {smoke_dir}/src
프로젝트 루트: {smoke_dir}

소스 코드를 분석하여 CLAUDE.md + IMPLEMENTS.md를 생성해주세요.
AskUserQuestion 사용하지 마세요. 코드 분석만으로 생성하세요.
""")
```

**검증 체크리스트:**

| 항목 | 검증 방법 |
|------|----------|
| CLAUDE.md 재생성 | `Read({smoke_dir}/src/CLAUDE.md)` 성공 |
| IMPLEMENTS.md 재생성 | `Read({smoke_dir}/src/IMPLEMENTS.md)` 성공 |
| Schema 검증 | `claude-md-core validate-schema --file {smoke_dir}/src/CLAUDE.md` 통과 |

### Phase 4: /validate 검증

drift-validator와 export-validator를 병렬 호출합니다.

```
# 단일 메시지에서 병렬 호출
Task(drift-validator, prompt="검증 대상: {smoke_dir}/src")
Task(export-validator, prompt="검증 대상: {smoke_dir}/src")
```

**검증 체크리스트:**

| 항목 | 검증 방법 |
|------|----------|
| Drift 검증 | drift-validator 결과에 status: approve 또는 에러 없이 완료 |
| Export 검증 | export-validator 결과에 status: approve 또는 에러 없이 완료 |

### Phase 5: Cleanup & Report

임시 디렉토리를 정리하고 최종 보고서를 생성합니다.

```
# 정리
Bash: rm -rf {smoke_dir}
```

## 결과 보고 형식

각 Phase의 PASS/FAIL을 아래 형식으로 보고합니다:

```
Smoke Test Report
=================

Phase 1: /impl  .............. {PASS|FAIL}
  - CLAUDE.md 생성: {PASS|FAIL}
  - IMPLEMENTS.md 생성: {PASS|FAIL}
  - Schema 검증: {PASS|FAIL}

Phase 2: /compile ............ {PASS|FAIL}
  - 소스 코드 생성: {PASS|FAIL}
  - 테스트 생성: {PASS|FAIL}

Phase 3: /decompile .......... {PASS|FAIL}
  - CLAUDE.md 재생성: {PASS|FAIL}
  - IMPLEMENTS.md 재생성: {PASS|FAIL}
  - Schema 검증: {PASS|FAIL}

Phase 4: /validate ........... {PASS|FAIL}
  - Drift 검증: {PASS|FAIL}
  - Export 검증: {PASS|FAIL}

Result: {N}/4 PASS
```

**실패 시:** 해당 항목 아래에 에러 메시지를 포함합니다.

```
Phase 1: /impl  .............. FAIL
  - CLAUDE.md 생성: PASS
  - IMPLEMENTS.md 생성: FAIL
    Error: IMPLEMENTS.md file not found at {smoke_dir}/src/IMPLEMENTS.md
  - Schema 검증: SKIP (Phase 1 실패로 건너뜀)
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| Phase N 실패 | 해당 Phase FAIL 표시, 이후 Phase는 SKIP |
| Agent timeout | FAIL + timeout 메시지 |
| Schema 검증 실패 | FAIL + 검증 에러 내용 포함 |
| 임시 디렉토리 생성 실패 | 즉시 중단 + 에러 보고 |

## 관련 컴포넌트

- `agents/impl-agent.md`: Phase 1 - CLAUDE.md + IMPLEMENTS.md 생성
- `agents/compiler.md`: Phase 2 - 소스 코드 + 테스트 생성
- `agents/decompiler.md`: Phase 3 - CLAUDE.md + IMPLEMENTS.md 재추출
- `agents/drift-validator.md`: Phase 4 - 코드-문서 일치 검증
- `agents/export-validator.md`: Phase 4 - Export 커버리지 검증
- `claude-md-core validate-schema`: Phase 1, 3 - 스키마 검증
