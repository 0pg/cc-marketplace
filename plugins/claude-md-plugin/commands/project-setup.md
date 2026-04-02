---
name: project-setup
description: |
  프로젝트/모듈 CLAUDE.md에 Conventions 섹션 (통합 6개 서브섹션)을 추가하거나 업데이트합니다.
  기존 프로젝트는 소스코드에서 컨벤션을 추출하고, 새 프로젝트는 대화형으로 수집합니다.
  --update 옵션으로 기존 Conventions 섹션을 수정할 수 있습니다 (기존 /convention-update 흡수).
argument-hint: "[project_root_path] [--update [content]]"
allowed-tools: [Bash, Read, Glob, Grep, Write, AskUserQuestion]
---

# /project-setup

프로젝트 CLAUDE.md에 Convention 섹션을 추가/업데이트하여 `/dev` REFACTOR 단계에서 참조할 수 있도록 합니다.

## Triggers

- `/project-setup`
- `프로젝트 설정`
- `컨벤션 생성`
- `컨벤션 업데이트` (--update 모드)
- `컨벤션 수정` (--update 모드)

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | 자동 탐지 | 프로젝트 루트 경로 |
| `--update` | 아니오 | false | 기존 Conventions 업데이트 모드 |
| `content` | 아니오 | - | --update 시 반영할 변경 지시사항 (없으면 대화형) |

## Workflow

### 1. 프로젝트 루트 결정

인자가 있으면 해당 경로 사용. 없으면 CWD에서 project root marker (`.git`, `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod` 등) 확인.

marker가 없으면 AskUserQuestion으로 경로 입력 요청.

**상위 디렉토리 탐색 금지** — CWD 외부로 탐색하지 않음.

### 2. 모듈 루트 탐지

build marker 파일 (`package.json`, `Cargo.toml`, `go.mod` 등) 기반 모듈 루트 자동 감지.
모듈 루트 미발견 시 프로젝트 루트를 싱글 모듈로 취급.

### 3. 기존 Convention 섹션 확인 + 모드 분기

project_root CLAUDE.md에서 `## Conventions` 존재 확인.

**`--update` 모드이거나 Conventions 존재 시:**
→ Step 3-U (업데이트 모드)로 분기

**Conventions 미존재 시:**
→ Step 4 (신규 생성)으로 진행

### 3-U. 업데이트 모드 (기존 /convention-update 흡수)

#### 3-U-A. 인자 content가 있는 경우

내용 분석으로 대상 서브섹션 자동 판별:
| 키워드 | 대상 |
|--------|------|
| 디렉토리, 폴더, 구조 | Project Structure |
| 모듈, 의존성, 레이어 | Module Boundaries |
| 패키지명, 디렉토리명 | Naming Conventions |
| 언어, 버전, 런타임 | Language & Runtime |
| 코딩, 패턴, 규칙 | Coding Rules |
| 변수명, 함수명, 네이밍 | Naming Rules |

대상 서브섹션에 content 반영 → 사용자 확인 → 저장.

#### 3-U-B. 인자 content가 없는 경우 (대화형)

현재 Conventions 6개 서브섹션을 표시하고 AskUserQuestion:
"수정할 서브섹션을 선택하세요: [1-6]"

선택된 서브섹션의 현재 내용을 표시하고 수정 내용을 수집.

완료 후 Step 9 (검증)으로 진행.

### 4. 프로젝트 유형 판별

소스 파일 존재 여부로 기존/신규 프로젝트 구분.

### 5. 컨벤션 추출 또는 수집

#### 5-A. 기존 프로젝트: 코드 분석으로 추출

| 분석 대상 | 방법 |
|-----------|------|
| 언어/런타임 | 파일 확장자 통계, 빌드 설정 파일 |
| 디렉토리 패턴 | 최상위 디렉토리 구조 분석 |
| 코딩 규칙 | 비동기 패턴, 에러 처리, 타입 사용 등 |
| 네이밍 규칙 | 변수/함수/클래스/상수 패턴 분석 |
| 테스트 패턴 | 프레임워크, 파일 패턴, Mock 전략 |

> **린트 제외 원칙**: 포맷터/린터 설정 파일이 존재하면 해당 도구가 처리하는 항목은 Convention에서 제외.

분석 결과를 사용자에게 보여주고 AskUserQuestion으로 확인.

#### 5-B. 신규 프로젝트: 대화형 수집

Q1. 언어 선택 → Q2. 구조 스타일 → Q3. 코딩 스타일

### 6. `## Instructions` 섹션 생성

project root CLAUDE.md에 `## Instructions`가 없으면 생성:

```markdown
## Instructions

- CLAUDE.md is the SSOT. Source code is a derived artifact generated from CLAUDE.md.
- When code disagrees with CLAUDE.md, regenerate code via /dev (not modify docs).
- To change requirements, update CLAUDE.md first, then code follows.
- Derive tests from DEVELOPERS.md Constraints.
- 소스코드는 /dev로 생성. Write tool로 직접 소스 파일 생성 금지.
- 완료 선언 전 /validate --strict 실행 필수.
```

### 7. `## Conventions` 섹션 생성

필수 6개 서브섹션 포함:

```markdown
## Conventions

### Project Structure
### Module Boundaries
### Naming Conventions
### Language & Runtime
### Coding Rules
### Naming Rules
```

선택적 서브섹션: API Design, Error Strategy, Testing Strategy, Test Convention 등.

### 8. 모듈별 Convention 처리 (DRY 원칙)

싱글 모듈이면 skip.
멀티 모듈: project_root와 동일하면 상속, 다르면 override 작성.

### 9. 검증

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
$CLI_PATH validate-convention --project-root {project_root}
```

CLI 빌드 실패 시 AskUserQuestion으로 설치/건너뛰기 질문.

### 10. 결과 보고

생성/업데이트된 파일 목록, 상속 정보 표시.

## 오류 처리

| 상황 | 대응 |
|------|------|
| 프로젝트 루트 탐지 실패 | 경로 입력 요청 |
| 파일 쓰기 권한 없음 | 에러 메시지 |
| 소스 분석 실패 | 대화형 수집으로 전환 |
| CLI 빌드 실패 | 설치/건너뛰기 질문 |
