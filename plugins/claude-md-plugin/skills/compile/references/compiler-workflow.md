# Compiler Agent - Detailed Workflow Reference
<!--
  This file contains the detailed phase-by-phase workflow, pseudocode blocks,
  skill invocation chain diagrams, and example tables extracted from agents/compiler.md.
  It is loaded at runtime by the compiler agent via cat command.
  Do not edit the main agent file's workflow without updating this file accordingly.
-->

## 워크플로우

### Phase 1: 컨텍스트 수집

#### 1.1 프로젝트 컨텍스트 로드

1. **프로젝트 root CLAUDE.md 읽기**: `.git` 또는 `package.json` 등으로 `project_root`를 탐지하고, build marker 기반으로 `module_root`를 탐색합니다. `{project_root}/CLAUDE.md`를 Read합니다.
2. **Convention 섹션 추출**: project CLAUDE.md에서 `## Project Convention` 섹션과 `## Code Convention` 섹션을 추출합니다 (project 기본값).
3. **Module override**: `module_root`가 `project_root`와 다르면 `{module_root}/CLAUDE.md`를 Read합니다. module CLAUDE.md에 `## Code Convention`이 있으면 이것으로 override합니다. `## Project Convention`이 있으면 이것으로도 override합니다.
4. **대상 CLAUDE.md 파싱**: `claude-md-core parse-claude-md` CLI를 호출합니다. 입력은 `claude_md_path`이며, 출력은 ClaudeMdSpec JSON (stdout)입니다. 파싱 결과를 `spec`에 저장합니다.
5. **IMPLEMENTS.md 읽기**: 대상 CLAUDE.md 경로에서 "CLAUDE.md"를 "IMPLEMENTS.md"로 치환한 경로의 파일을 읽습니다. 파일이 존재하면 파싱하여 `implements_spec`에 저장합니다. 존재하지 않으면 기본 Planning Section 템플릿으로 자동 생성한 후 파싱합니다.

**CLAUDE.md (WHAT)**에서 추출:
- `exports`: 함수, 타입, 클래스 정의
- `behaviors`: 동작 시나리오 (테스트 케이스로 변환)
- `contracts`: 사전/사후조건 (검증 로직으로 변환)
- `dependencies`: 필요한 import문 생성
- `domain_context`: 코드 생성 결정에 반영할 맥락 (결정 근거, 제약, 호환성)

**IMPLEMENTS.md Planning Section (HOW)**에서 추출:
- `dependencies_direction`: 의존성 위치와 사용 목적
- `implementation_approach`: 구현 전략과 대안
- `technology_choices`: 기술 선택 근거

**중요**: 코드 생성 시 CLAUDE.md 내 Convention 섹션의 규칙을 우선 따르고, 없으면 `project_claude_md` 일반 내용을 fallback으로 참조합니다.
`implements_spec`의 구현 방향도 함께 참조합니다.

**컨벤션 참조 우선순위**:
1. `module_root` CLAUDE.md `## Code Convention` → 코드 스타일, 네이밍 규칙
2. `module_root` CLAUDE.md `## Project Convention` (optional override) → 구조 규칙
3. `project_root` CLAUDE.md `## Code Convention` (default) → 코드 스타일 fallback
4. `project_root` CLAUDE.md `## Project Convention` → 구조 규칙
5. `project_root` CLAUDE.md 일반 내용 → 최종 fallback

#### 1.2 의존성 인터페이스 탐색

Dependencies 섹션의 internal 항목은 CLAUDE.md 경로가 명시되어 있으므로 직접 읽습니다.

Dependencies 섹션의 각 internal 항목에 대해 다음을 수행합니다:

1. `dep.path`로 명시된 CLAUDE.md를 직접 Read합니다 (경로가 deterministic하므로 검색 불필요).
2. Exports 섹션을 추출하여 `dependency_interfaces`에 저장합니다.
3. 필요시 Behavior 섹션을 추가로 읽어 동작을 이해합니다.

실제 소스코드 탐색은 최후 수단입니다. Exports 시그니처만으로 구현 불가능한 경우에만 소스코드를 참조합니다.

**탐색 우선순위**:

| 우선순위 | 단계 | 탐색 대상 | 획득 정보 |
|----------|------|-----------|----------|
| 1 (필수) | 대상 CLAUDE.md Dependencies | 의존 모듈 CLAUDE.md 경로 | 어떤 모듈에 의존하는지 |
| 2 (필수) | 의존 모듈 CLAUDE.md Exports | 인터페이스 카탈로그 | 함수/타입/클래스 시그니처 |
| 3 (선택) | 의존 모듈 CLAUDE.md Behavior | 동작 이해 | 정상/에러 시나리오 |
| 4 (최후) | 실제 소스코드 | 구현 세부사항 | Exports만으로 불충분할 때만 |

**금지 사항**:

- 소스코드를 먼저 탐색하지 않습니다.
- Exports를 무시하고 구현 파일을 직접 읽지 않습니다.

**이유**: CLAUDE.md의 Exports는 **Interface Catalog**로서 설계되었습니다.
코드 탐색보다 CLAUDE.md 탐색이 더 효율적이고, 캡슐화 원칙을 준수합니다.

#### 1.3 Domain Context 반영

**Domain Context는 compile 재현성의 핵심입니다.** 동일한 CLAUDE.md에서 동일한 코드를 생성하려면 Domain Context의 값들이 코드에 그대로 반영되어야 합니다.

Domain Context가 있으면 다음 규칙에 따라 코드에 반영합니다:

1. **Decision Rationale → 상수 값 결정**: 각 결정 근거에서 상수 값을 추출하여 코드에 반영합니다. 예: "TOKEN_EXPIRY: 7일 (PCI-DSS)" → `const TOKEN_EXPIRY_DAYS = 7`
2. **Constraints → 검증 로직 강화**: 각 제약 조건에서 검증 로직을 생성합니다. 예: "비밀번호 재설정 90일" → `validatePasswordAge(90)`
3. **Compatibility → 레거시 지원 코드**: 각 호환성 요구에서 레거시 지원 코드를 포함합니다. 예: "UUID v1 지원" → `parseUUIDv1()` 함수 포함

**Domain Context 반영 예시**:

| Domain Context | 생성 코드 |
|----------------|----------|
| `TOKEN_EXPIRY: 7일 (PCI-DSS)` | `const TOKEN_EXPIRY_DAYS = 7; // PCI-DSS compliance` |
| `TIMEOUT: 2000ms (IdP SLA × 4)` | `const TIMEOUT_MS = 2000; // Based on IdP SLA` |
| `MAX_RETRY: 3 (외부 API SLA)` | `const MAX_RETRY = 3;` |
| `UUID v1 지원 필요` | UUID v1 파싱 로직 포함 |
| `동시 세션 최대 5개` | 세션 수 검증 로직 포함 |

### Phase 2: 언어 감지 확인

감지된 언어가 없으면 다음 절차를 따릅니다:

1. 대상 디렉토리의 파일 확장자를 기반으로 언어를 자동 감지합니다.
2. 자동 감지가 불가능하면 AskUserQuestion으로 사용자에게 질문합니다. 옵션은 프로젝트에서 사용 중인 언어 목록으로 동적 생성합니다.

### Phase 3: TDD 워크플로우 (내부 자동 수행)

#### 3.1 RED Phase - 테스트 생성

Behaviors를 기반으로 테스트 파일을 생성합니다. 테스트 프레임워크는 프로젝트 설정(package.json, pyproject.toml 등)에서 감지하며, 감지 불가 시 `project_claude_md`에 명시된 프레임워크를 사용합니다.

각 behavior에 대해:
- `success` 카테고리이면 성공 케이스 테스트를 생성합니다.
- 그 외이면 에러 케이스 테스트를 생성합니다.

테스트 생성 시:
- 프로젝트 CLAUDE.md의 테스트 프레임워크/컨벤션을 따름
- 명시되지 않은 경우 해당 언어의 표준 테스트 프레임워크 사용

#### 3.2 GREEN Phase - 구현 + 테스트 통과

Exports와 contracts를 기반으로 구현 파일을 생성하고, 테스트가 통과할 때까지 반복합니다:

1. **타입/인터페이스 파일 생성**: `spec.exports.types`에서 타입 파일을 생성합니다.
2. **에러 클래스 파일 생성**: behaviors에서 에러 타입을 추출하여 에러 파일을 생성합니다.
3. **메인 구현 파일 생성**: 각 함수에 대해 시그니처, contracts, behaviors를 기반으로 구현을 생성합니다.
4. **테스트 실행 및 반복**: 테스트를 실행하고, 실패하면 실패한 테스트를 분석하여 구현을 수정한 후 재실행합니다. 최대 3회 재시도합니다. 3회 재시도 후에도 실패하면 경고를 기록합니다.

#### 3.3 REFACTOR Phase - 코드 개선

테스트가 모두 통과하면 CLAUDE.md Convention 섹션의 규칙에 맞게 리팩토링합니다. Convention 섹션이 없으면 project CLAUDE.md를 fallback으로 참조합니다:
- `## Code Convention`: 코드 스타일, 네이밍 규칙 (PRIMARY)
- `## Project Convention`: 구조 규칙, 모듈 경계 (PRIMARY)
- `project_claude_md` 일반 내용: FALLBACK

리팩토링 후 테스트를 재실행하여 회귀를 확인합니다. 리팩토링으로 테스트가 실패하면 롤백합니다.

### Phase 4: 파일 충돌 처리

생성된 각 파일에 대해 대상 경로에 파일이 이미 존재하는지 확인합니다:
- `conflict_mode`가 "skip"이면 기존 파일을 유지하고 건너뜁니다.
- `conflict_mode`가 "overwrite"이면 기존 파일을 덮어씁니다.
- 존재하지 않으면 새 파일을 생성합니다.

### Phase 5: IMPLEMENTS.md Implementation Section 업데이트

코드 생성 과정에서 발견된 정보를 수집하여 IMPLEMENTS.md의 Implementation Section을 업데이트합니다:

1. 생성된 코드에서 다음 정보를 추출합니다:
   - Algorithm: 복잡한 로직만 추출
   - Key Constants: 도메인 의미가 있는 상수만 추출
   - Error Handling: 에러 처리 패턴
   - State Management: 상태 관리 패턴
   - Implementation Guide: 변경 사항 기록

2. `{target_dir}/IMPLEMENTS.md`를 Read하여 기존 내용을 로드합니다.
3. Implementation Section을 업데이트한 후 Write합니다.

#### Implementation Section 업데이트 규칙

| 섹션 | 업데이트 조건 | 내용 |
|------|--------------|------|
| Algorithm | 복잡하거나 비직관적인 로직이 있을 때 | 구현 단계, 특수 처리 |
| Key Constants | 도메인 의미가 있는 상수가 있을 때 | 이름, 값, 근거, 영향 범위 |
| Error Handling | 에러 처리가 있을 때 | 에러 타입, 재시도, 복구, 로그 레벨 |
| State Management | 상태 관리가 있을 때 | 초기 상태, 저장, 정리 |
| Implementation Guide | 구현 중 특이사항이 있을 때 | 날짜, 변경 사항, 이유 |

### Phase 6: 결과 반환

다음 구조의 결과 JSON을 생성하여 파일에 저장합니다:

```json
{
  "claude_md_path": "{claude_md_path}",
  "implements_md_path": "{implements_md_path}",
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
  "implements_md_updated": true,
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
implements_md_updated: true
---end-compiler-result---
```

## 파일 구조 결정

**CLAUDE.md `## Project Convention > ### Project Structure` 섹션을 우선 따르고, 없으면 프로젝트 root CLAUDE.md의 Structure 섹션을 따릅니다.**

프로젝트 CLAUDE.md에 Structure가 명시되지 않은 경우:
1. 기존 프로젝트 파일 구조를 분석하여 패턴 추론
2. 해당 언어의 일반적인 컨벤션 적용

## Skill 호출 체인

```
┌─────────────────────────────────────────────────────────────┐
│                     compiler Agent                          │
│                                                              │
│  ┌─ Read(project_root/CLAUDE.md) ────────────────────────┐ │
│  │ 프로젝트 코딩 컨벤션, 구조 규칙 수집                    │ │
│  │  - ## Project Convention (구조 규칙)                    │ │
│  │  - ## Code Convention (코드 스타일)                     │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Read(module_root/CLAUDE.md) Convention sections ────┐ │
│  │ module_root != project_root 시 override 로드         │ │
│  │  - ## Code Convention (override)                      │ │
│  │  - ## Project Convention (optional override)          │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Bash(claude-md-core parse-claude-md) ──────────────────┐ │
│  │ 대상 CLAUDE.md → ClaudeMdSpec JSON (WHAT)              │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Read(IMPLEMENTS.md) ─────────────────────────────────┐ │
│  │ Planning Section 로드 (HOW direction)                  │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 언어 감지 (또는 AskUserQuestion) ─────────────────────┐ │
│  │ 대상 디렉토리 파일 확장자 기반 언어 결정               │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ TDD Workflow (내부 자동) ────────────────────────────┐ │
│  │                                                        │ │
│  │  [RED] behaviors → 테스트 파일 생성 (실패 확인)       │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [GREEN] 구현 생성 + 테스트 통과 (최대 3회 재시도)    │ │
│  │         └─ CLAUDE.md + IMPLEMENTS.md Planning 참조    │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [REFACTOR] Convention 섹션 기반 코드 정리             │ │
│  │         └─ Convention sections > project CLAUDE.md   │ │
│  │         └─ 회귀 테스트로 안전성 확인                  │ │
│  │                                                        │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 파일 충돌 처리 ──────────────────────────────────────┐ │
│  │ skip (기본) 또는 overwrite 모드                        │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ IMPLEMENTS.md Implementation Section 업데이트 ───────┐ │
│  │ Algorithm, Key Constants, Error Handling 등 기록       │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 결과 반환 ───────────────────────────────────────────┐ │
│  │ 생성된 파일 목록, 테스트 결과, 상태                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```
