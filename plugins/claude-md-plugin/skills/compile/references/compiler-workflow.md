# Compiler Agent - Detailed Workflow Reference
<!--
  This file contains the detailed phase-by-phase workflow for the compiler agent.
  The compiler handles the full Inline TDD cycle: RED (test generation) → GREEN (implementation) → REFACTOR.
  Loaded at runtime by the compiler agent via cat command.

  v7: CLAUDE.md provides Requirements + Domain Context. DEVELOPERS.md provides Constraints (test source) + Technical Context.
  compile-context provides ephemeral session spec (implementation approach, dependencies).
-->

## 워크플로우

### Phase 1: 컨텍스트 수집

#### 1.1 프로젝트 컨텍스트 로드

1. **프로젝트 root CLAUDE.md 읽기**: `.git` 또는 `package.json` 등으로 `project_root`를 탐지하고, build marker 기반으로 `module_root`를 탐색합니다. `{project_root}/CLAUDE.md`를 Read합니다.
2. **Convention 섹션 추출**: project CLAUDE.md에서 `## Conventions` 섹션을 추출합니다 (project 기본값).
3. **Module override**: `module_root`가 `project_root`와 다르면 `{module_root}/CLAUDE.md`를 Read합니다. module CLAUDE.md에 `## Conventions`이 있으면 project_root의 canonical source를 override합니다 (없으면 project_root에서 상속).
4. **대상 CLAUDE.md 읽기**: 대상 CLAUDE.md를 Read합니다. v7 스키마에서 추출하는 정보:
   - **Requirements**: 비즈니스 요구사항 (PM 관점, 고수준 검증 기준)
   - **Domain Context**: 비즈니스 제약 배경 (규정, 레거시, 조직적 이유)
5. **compile-context 읽기**: 대상 CLAUDE.md 경로에서 "CLAUDE.md"를 "compile-context.md"로 치환한 경로의 파일을 읽습니다. 파일이 존재하면 파싱하여 `compile_context`에 저장합니다. 존재하지 않으면 이 단계를 건너뜁니다 (compile-context는 optional).

**CLAUDE.md (Primary SSOT -- PM 요구사항)**에서 추출:
- `requirements`: 비즈니스 요구사항 (사용자 관점, 고수준 검증 기준)
- `domain_context`: 코드 생성 결정에 반영할 맥락 (규정, 레거시, 조직적 이유)

**compile-context (세션 임시 스펙, optional)**에서 추출:
- `dependencies_direction`: 의존성 위치와 사용 목적
- `implementation_approach`: 구현 전략과 대안
- `technology_choices`: 기술 선택 근거

**DEVELOPERS.md (Derived Spec -- 테스트 생성 원천)**에서 선택적 참조:
- `constraints`: 정밀한 입출력 계약 → **테스트 생성의 주요 원천**
- `technical_context`: 기술 선택 + 근거 → 구현 방식 결정
- `decision_log`: ADR 스타일 결정 근거 → 구현 방식 결정에 참고
- `operations`: 배포, 모니터링, gotchas → 운영 고려사항 참고
- **참조 조건**: DEVELOPERS.md가 존재하고, 해당 섹션이 `None`이 아닌 경우에만 참조
- **우선순위**: CLAUDE.md > DEVELOPERS.md — 충돌 시 CLAUDE.md가 우선

6. **DEVELOPERS.md 선택적 읽기**: 대상 CLAUDE.md 경로에서 "CLAUDE.md"를 "DEVELOPERS.md"로 치환한 경로의 파일을 읽습니다. 파일이 존재하면 `Constraints`, `Technical Context`, `Decision Log`, `Operations` 섹션을 `developers_context`에 저장합니다. 존재하지 않거나 섹션이 `None`이면 이 단계를 건너뜁니다 (DEVELOPERS.md는 optional이지만, 존재하면 Constraints가 테스트 생성의 주요 원천).

**중요**: `project_root` CLAUDE.md의 Conventions가 canonical source입니다. `module_root`에 Conventions가 있으면 override로 사용합니다. Convention 섹션이 없으면 `project_claude_md` 일반 내용을 fallback으로 참조합니다.

**컨벤션 참조 우선순위**:
1. `module_root` CLAUDE.md `## Conventions` (override, project_root와 다를 때만 존재)
2. `project_root` CLAUDE.md `## Conventions` (canonical source)
3. `project_root` CLAUDE.md 일반 내용 → 최종 fallback

#### 1.2 의존성 인터페이스 탐색

compile-context의 Dependencies Direction 섹션에서 의존성 정보를 확인합니다.

**탐색 우선순위:**

| 우선순위 | 단계 | 탐색 대상 | 획득 정보 |
|----------|------|-----------|----------|
| 1 (필수) | compile-context Dependencies Direction | 의존 모듈 CLAUDE.md 경로 | 어떤 모듈에 의존하는지 |
| 2 (선택) | 의존 모듈 CLAUDE.md | Purpose, Requirements | 의존 모듈의 역할과 요구사항 |
| 3 (선택) | 의존 모듈 소스코드 | 인터페이스 | compile-context가 부족할 때만 |

compile-context가 없으면 기존 소스코드를 직접 탐색하여 import/export 관계를 파악합니다.

#### 1.3 Requirements + Constraints → 코드 변환

**Requirements (CLAUDE.md)와 Constraints (DEVELOPERS.md)가 검증 가능한 코드로 변환됩니다.** DEVELOPERS.md Constraints가 있으면 정밀한 입출력 계약을 기반으로 변환하고, 없으면 CLAUDE.md Requirements에서 fallback합니다.

| DEVELOPERS.md Constraints (정밀) | 생성 코드 |
|----------------------------------|----------|
| `TokenService.issue(user) → token.expiresAt <= now + 7d` | `const MAX_TOKEN_EXPIRY_DAYS = 7;` + 검증 로직 |
| `SessionManager.create(userId) throws MaxSessionError when active >= 5` | `const MAX_SESSIONS = 5;` + 세션 수 검증 |
| `CsvParser.parse(input) requires input.encoding == UTF-8` | 인코딩 검증 guard clause |
| `TokenStore.save(token) requires storage.isSecure == true` | storage 추상화 + secure 검증 |

| CLAUDE.md Requirements (fallback) | 생성 코드 |
|-----------------------------------|----------|
| `토큰은 발급 후 7일 이내에 만료되어야 한다` | `const MAX_TOKEN_EXPIRY_DAYS = 7;` + 검증 로직 |
| `사용자당 동시 활성 세션은 5개로 제한한다` | `const MAX_SESSIONS = 5;` + 세션 수 검증 |

#### 1.4 Domain Context + Technical Context → 코드 반영

**Domain Context (CLAUDE.md)는 compile 재현성의 핵심입니다.** 동일한 CLAUDE.md에서 동일한 코드를 생성하려면 Domain Context의 값들이 코드에 그대로 반영되어야 합니다.

| CLAUDE.md Domain Context | 생성 코드 |
|--------------------------|----------|
| `PCI-DSS 준수를 위해 7일 만료` | `const TOKEN_EXPIRY_DAYS = 7; // PCI-DSS compliance` |
| `UUID v1 지원 필요` | UUID v1 파싱 로직 포함 |

**Technical Context (DEVELOPERS.md)는 기술적 구현 결정에 반영됩니다.** 기술 선택과 근거를 코드에 반영합니다.

| DEVELOPERS.md Technical Context | 생성 코드 |
|---------------------------------|----------|
| `IdP SLA 500ms, 타임아웃 = SLA × 4` | `const TIMEOUT_MS = 2000; // Based on IdP SLA` |
| `Redis 캐시 사용, TTL = token 만료 - 10min` | Redis 캐시 설정 + TTL 계산 로직 |

### Phase 2: 테스트 생성 (RED)

DEVELOPERS.md Constraints에서 테스트를 생성합니다. DEVELOPERS.md가 없으면 CLAUDE.md Requirements에서 fallback합니다.

#### 2.1 Constraints → 테스트 매핑

DEVELOPERS.md의 각 Constraint를 테스트 케이스로 변환합니다 (DEVELOPERS.md 부재 시 CLAUDE.md Requirements에서 fallback):

1. **수치 제한** → 경계값 테스트
   - `"token.expiresAt <= now + 7d"` → `test: 7일 OK, 8일 실패`
2. **형식 제약** → 유효/무효 입력 테스트
   - `"input.encoding == UTF-8"` → `test: UTF-8 OK, non-UTF-8 실패`
3. **비즈니스 규칙** → 규칙 준수/위반 시나리오
   - `"throws DuplicateError when exists"` → `test: 중복 시 에러`

#### 2.2 Technical Context → 경계값/상수 추출

DEVELOPERS.md Technical Context에서 테스트에 사용할 구체적인 값을 추출합니다:

| Technical Context | 추출 값 | 테스트 활용 |
|-------------------|---------|-----------|
| `IdP SLA 500ms, 타임아웃 = SLA × 4` | `2000` | 타임아웃 경계 테스트 |
| `PCI-DSS 7일 만료` | `7` | 만료 경계 테스트 |

#### 2.3 기존 소스 참조 (overwrite 모드)

`overwrite` 모드에서 기존 소스가 있으면:
```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
$CLI_PATH analyze-code --path {target_dir}
```
인터페이스를 발견하여 테스트 호환성 보장.

#### 2.4 테스트 파일 생성

언어별 테스트 프레임워크에 맞는 테스트 파일을 Write합니다:

```
describe('Constraints Tests', () => {
  // DEVELOPERS.md Constraints 기반 테스트 (fallback: CLAUDE.md Requirements)
});

describe('Technical Context Tests', () => {
  // Technical Context 경계값/상수 검증 테스트
});
```

테스트가 실패하는지 확인 (RED 상태):
```bash
# 테스트 실행 — 아직 구현이 없으므로 실패해야 함
{test_command}
```

### Phase 3: GREEN Phase - 구현 + 테스트 통과

Requirements + Constraints와 Domain Context + Technical Context를 기반으로 구현 파일을 생성하고, 테스트가 통과할 때까지 반복합니다:

1. **타입/인터페이스 파일 생성**: compile-context 또는 기존 코드에서 필요한 타입을 추출하여 생성합니다.
2. **메인 구현 파일 생성**: DEVELOPERS.md Constraints를 검증 로직으로 (fallback: CLAUDE.md Requirements), Technical Context를 상수/설정으로 (fallback: Domain Context) 변환하여 구현합니다.
3. **테스트 실행 및 반복**: 테스트를 실행하고, 실패하면 실패한 테스트를 분석하여 **구현을 수정**한 후 재실행합니다. 최대 3회 재시도합니다. 3회 재시도 후에도 실패하면 경고를 기록합니다.

**테스트 수정 원칙**: 테스트가 실패하면 구현 코드를 수정합니다. 단, 자신이 Phase 2에서 생성한 테스트이므로, 테스트 자체에 명백한 오류(잘못된 import 경로, 오타 등)가 있으면 수정 가능합니다. 다만 Constraints에서 도출한 assertion 로직은 변경하지 않습니다.

### Phase 4: REFACTOR Phase - 코드 개선

테스트가 모두 통과하면 CLAUDE.md Conventions 섹션의 규칙에 맞게 리팩토링합니다. Conventions 섹션이 없으면 project CLAUDE.md를 fallback으로 참조합니다:
- `## Conventions`: 코딩 규칙, 구조 규칙, 네이밍 규칙 (PRIMARY)
- `project_claude_md` 일반 내용: FALLBACK

리팩토링 후 테스트를 재실행하여 회귀를 확인합니다. 리팩토링으로 테스트가 실패하면 롤백합니다.

### Phase 5: 파일 충돌 처리

생성된 각 파일에 대해 대상 경로에 파일이 이미 존재하는지 확인합니다:
- `conflict_mode`가 "skip"이면 기존 파일을 유지하고 건너뜁니다.
- `conflict_mode`가 "overwrite"이면 기존 파일을 덮어씁니다.
- 존재하지 않으면 새 파일을 생성합니다.

### Phase 6: compile-context 업데이트 (session temp)

코드 생성 과정에서 발견된 정보를 수집하여 compile-context를 업데이트합니다. compile-context는 세션 임시 파일로, 다음 compile 시 참고용입니다:

1. 생성된 코드에서 다음 정보를 추출합니다:
   - Implementation Approach: 구현 전략
   - Key Constants: 도메인 의미가 있는 상수
   - Error Handling: 에러 처리 패턴

2. `.claude/tmp/compile-context-{dir-hash}.md`가 존재하면 Read하여 기존 내용을 로드합니다.
3. compile-context를 업데이트한 후 Write합니다.

### Phase 7: 결과 반환

다음 구조의 결과 JSON을 생성하여 파일에 저장합니다:

```json
{
  "claude_md_path": "{claude_md_path}",
  "compile_context_path": "{compile_context_path}",
  "target_dir": "{target_dir}",
  "detected_language": "{detected_language}",
  "generated_files": ["{written_files}"],
  "skipped_files": ["{skipped_files}"],
  "overwritten_files": ["{overwritten_files}"],
  "tests": {
    "total": "{total}",
    "passed": "{passed}",
    "failed": "{failed}"
  },
  "compile_context_updated": true,
  "status": "success | warning"
}
```

`status`는 모든 테스트가 통과하면 "success", 실패가 있으면 "warning"입니다. 다음 형식의 결과 블록을 출력합니다:

```
---compiler-result---
result_file: {result_file}
status: {status}
generated_files: {written_files}
skipped_files: {skipped_files}
tests_passed: {passed}
tests_failed: {failed}
compile_context_updated: true
---end-compiler-result---
```

## 파일 구조 결정

**CLAUDE.md `## Conventions > ### Project Structure` 섹션을 우선 따르고, 없으면 프로젝트 root CLAUDE.md의 구조를 따릅니다.**

프로젝트 CLAUDE.md에 Project Structure가 명시되지 않은 경우:
1. 기존 프로젝트 파일 구조를 분석하여 패턴 추론
2. 해당 언어의 일반적인 컨벤션 적용

## Skill 호출 체인

```
┌─────────────────────────────────────────────────────────────┐
│                     compiler Agent                          │
│                                                             │
│  ┌─ Read(project_root/CLAUDE.md) ────────────────────────┐ │
│  │ 프로젝트 코딩 컨벤션, 구조 규칙 수집                   │ │
│  │  - ## Conventions (구조 + 코딩 규칙)                   │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ Read(module_root/CLAUDE.md) Convention sections ────┐ │
│  │ module_root != project_root 시 override 로드         │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ Read(target CLAUDE.md) ─────────────────────────────┐ │
│  │ Requirements + Domain Context 추출                    │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ Read(compile-context.md) ───────────────────────────┐ │
│  │ 세션 스펙 로드 (optional): 의존성, 구현 전략          │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ Read(DEVELOPERS.md) ───────────────────────────────┐  │
│  │ Constraints + Technical Context 추출 (테스트 원천)   │  │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ RED + GREEN + REFACTOR Workflow ─────────────────────┐ │
│  │                                                        │ │
│  │  [RED] DEVELOPERS.md Constraints → 테스트 생성        │ │
│  │         └─ 수치 제한 → 경계값 테스트                  │ │
│  │         └─ 형식 제약 → 유효/무효 입력 테스트           │ │
│  │         └─ Technical Context → 상수/경계값 추출        │ │
│  │         └─ (fallback: CLAUDE.md Requirements)         │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [GREEN] 구현 생성 + 테스트 통과 (최대 3회 재시도)     │ │
│  │         └─ Constraints → 검증 로직 (DEVELOPERS.md)    │ │
│  │         └─ Technical Context → 상수/설정              │ │
│  │         └─ compile-context → 구현 전략 (optional)     │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [REFACTOR] Convention 섹션 기반 코드 정리             │ │
│  │         └─ Convention sections > project CLAUDE.md     │ │
│  │         └─ 회귀 테스트로 안전성 확인                   │ │
│  │                                                        │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ 파일 충돌 처리 ──────────────────────────────────────┐ │
│  │ skip (기본) 또는 overwrite 모드                        │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ compile-context 업데이트 (session temp) ─────────────┐ │
│  │ Implementation Approach, Key Constants 등 기록         │ │
│  └───────────────────────┬───────────────────────────────┘ │
│                          │                                  │
│                          ▼                                  │
│  ┌─ 결과 반환 ───────────────────────────────────────────┐ │
│  │ 생성된 파일 목록, 테스트 결과, 상태                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```
