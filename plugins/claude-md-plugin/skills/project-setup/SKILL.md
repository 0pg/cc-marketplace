---
name: project-setup
version: 1.2.0
aliases: [setup, build-setup]
trigger:
  - /project-setup
  - 프로젝트 설정
  - 빌드 설정
description: |
  This skill should be used when the user asks to "set up project", "configure build commands",
  "detect test commands", or uses "/project-setup".
  Auto-detects build/test/lint commands from project configuration files and persists them to CLAUDE.md.
  Also analyzes existing code to generate code-convention.md with coding style/convention guide.
allowed-tools: [Bash, Read, Glob, Write, AskUserQuestion, Grep]
---

# Project Setup Skill

<example>
<context>
사용자가 새 프로젝트에서 처음 설정을 진행합니다.
</context>
<user>/project-setup</user>
<assistant_response>
프로젝트 파일을 분석합니다...

## Detected Commands

- **Build**: `pnpm build`
- **Test**: `pnpm test`
- **Lint**: `pnpm lint`

## Detected Code Convention

소스 파일 42개 분석 완료

- **Naming**: Variables=camelCase, Functions=camelCase, Types=PascalCase
- **Formatting**: 2 spaces, single quotes, no semicolons
- **Imports**: external → internal → relative

이 설정을 CLAUDE.md와 code-convention.md에 저장할까요?
</assistant_response>
</example>

<example>
<context>
사용자가 Rust 프로젝트에서 설정을 진행합니다.
</context>
<user>/project-setup</user>
<assistant_response>
프로젝트 파일을 분석합니다...

감지된 설정 파일: Cargo.toml

## Detected Commands

- **Build**: `cargo build`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`

이 설정을 CLAUDE.md에 저장할까요?
</assistant_response>
</example>

<example>
<context>
사용자 프로젝트에 설정 파일이 없는 경우.
</context>
<user>/project-setup</user>
<assistant_response>
프로젝트 파일을 분석합니다...

⚠ 설정 파일을 찾을 수 없습니다 (package.json, Makefile, Cargo.toml 등).

빌드/테스트 커맨드를 직접 입력해주세요:

[AskUserQuestion으로 Build, Test, Lint 커맨드 입력 요청]
</assistant_response>
</example>

## 목적

프로젝트의 빌드/테스트/린트 커맨드를 자동 감지하고 CLAUDE.md에 저장합니다.
또한 기존 소스 코드를 분석하여 코딩 스타일/컨벤션 가이드(`code-convention.md`)를 생성합니다.

## 워크플로우

### Phase 1: 프로젝트 파일 탐색

프로젝트 루트 디렉토리의 파일 목록을 확인합니다.

### Phase 2: 빌드 커맨드 추론

설정 파일을 순서대로 확인하여 커맨드를 추론합니다:

| 파일 | 프로젝트 타입 | 추론 방법 |
|------|-------------|----------|
| `package.json` | Node.js | scripts 섹션 파싱 |
| `Makefile` | Make | 타겟 목록 추출 |
| `Cargo.toml` | Rust | cargo build/test/clippy |
| `go.mod` | Go | go build/test |
| `pyproject.toml` | Python | pytest, ruff 등 |
| `build.gradle` / `pom.xml` | Java | gradle/maven 커맨드 |

설정 파일이 없으면 사용자에게 직접 입력 요청합니다.

### Phase 3: 사용자 확인

감지된 커맨드를 표시하고 AskUserQuestion으로 확인합니다:

```
## Detected Commands

- **Build**: `{build_command}`
- **Test**: `{test_command}`
- **Lint**: `{lint_command}`

이 설정을 CLAUDE.md에 저장할까요?
```

### Phase 4: 코드 컨벤션 분석

기존 소스 코드가 존재하면 코딩 스타일/컨벤션을 분석합니다.

#### 4.1 소스 파일 수집

Glob으로 소스 파일을 수집합니다:

```
Glob("**/*.{ts,tsx,js,jsx,py,rs,go,java,kt}", path={project_root})
```

소스 파일이 없으면 컨벤션 분석을 건너뛰고 사용자에게 알립니다.

#### 4.2 패턴 분석

수집된 소스 파일에서 다음 패턴을 추출합니다:

| 분석 항목 | 추출 방법 |
|----------|----------|
| Naming (변수) | 변수 선언 패턴 → camelCase / snake_case 비율 |
| Naming (함수) | 함수 선언 패턴 → camelCase / snake_case 비율 |
| Naming (타입) | 타입/클래스 선언 패턴 → PascalCase 확인 |
| Naming (상수) | 상수 선언 패턴 → UPPER_SNAKE_CASE 확인 |
| Naming (파일) | 파일명 패턴 → kebab-case / snake_case / PascalCase |
| Formatting | 들여쓰기 (spaces 수 / tab), 따옴표 (single / double) |
| Code Structure | import 순서, export 스타일 |
| Error Handling | 에러 처리 패턴 (throw / return / raise) |
| Testing | 테스트 프레임워크, 파일 패턴 (*.test.* / *.spec.* / test_*) |

**분석 방법:**
- 샘플링: 최대 20개 소스 파일을 읽어 패턴 추출
- 다수결: 70% 이상 동일 패턴이면 해당 패턴으로 결정
- 불명확: 패턴이 혼재하면 사용자에게 AskUserQuestion으로 확인

#### 4.3 사용자 확인

분석 결과를 표시하고 AskUserQuestion으로 확인합니다:

```
## Detected Code Convention

소스 파일 {N}개 분석 완료

- **Naming**: Variables={pattern}, Functions={pattern}, Types={pattern}
- **Formatting**: {indentation}, {quotes}, {semicolons}
- **Imports**: {order}

이 컨벤션을 code-convention.md에 저장할까요?
```

### Phase 5: CLAUDE.md 저장

확인된 커맨드를 CLAUDE.md에 저장합니다:

```markdown
# Build and Test Commands

- **Build**: `pnpm build`
- **Test**: `pnpm test`
- **Lint**: `pnpm lint`

# Workflow
- Run tests after making code changes
- Ensure linting passes before committing
```

**업데이트 로직:**
1. CLAUDE.md가 없으면 새로 생성
2. "Build and Test Commands" 섹션이 없으면 파일 시작 부분에 추가
3. 섹션이 있으면 **부족한 항목만 추가** (기존 항목 유지)

**병합 규칙:**

| 항목 | 기존 값 있음 | 기존 값 없음 |
|------|------------|------------|
| Build | 유지 | 감지된 값 추가 |
| Test | 유지 | 감지된 값 추가 |
| Lint | 유지 | 감지된 값 추가 |

**예시:**

기존 CLAUDE.md:
```markdown
# Build and Test Commands
- **Build**: `make build`
```

감지된 커맨드: Build=`pnpm build`, Test=`pnpm test`, Lint=`pnpm lint`

결과:
```markdown
# Build and Test Commands
- **Build**: `make build`  ← 기존 유지
- **Test**: `pnpm test`    ← 새로 추가
- **Lint**: `pnpm lint`    ← 새로 추가
```

### Phase 6: code-convention.md 저장

컨벤션 분석 결과가 확인되면 프로젝트 루트에 `code-convention.md`를 생성합니다.

스키마는 `templates/code-convention-schema.md`를 따릅니다.

**업데이트 로직:**
1. code-convention.md가 없으면 새로 생성
2. 이미 존재하면 기존 내용을 유지하고 부족한 항목만 추가 (Phase 5 CLAUDE.md와 동일 병합 규칙)

**출력 예시:**

```markdown
# Code Convention

## Naming

### Variables
- pattern: camelCase
- examples: userId, tokenExpiry, isValid

### Functions
- pattern: camelCase
- prefix: verb
- examples: getUserById, validateToken

### Types/Interfaces
- pattern: PascalCase
- examples: User, AuthConfig

### Constants
- pattern: UPPER_SNAKE_CASE
- examples: MAX_RETRY_COUNT, API_TIMEOUT_MS

### Files
- pattern: kebab-case
- test_suffix: .test
- examples: auth-service.ts

## Formatting

- indentation: 2 spaces
- line_length: 100
- quotes: single
- semicolons: false

## Code Structure

### Imports
- order: [builtin, external, internal, relative]
- grouping: true

### Exports
- style: named

## Testing

- framework: vitest
- file_pattern: *.test.ts
```

## 출력 형식

### 성공 시

```
✓ CLAUDE.md 업데이트 완료

저장된 커맨드:
- Build: `pnpm build`
- Test: `pnpm test`
- Lint: `pnpm lint`

파일: /path/to/project/CLAUDE.md

✓ code-convention.md 생성 완료

감지된 컨벤션:
- Naming: camelCase (variables/functions), PascalCase (types)
- Formatting: 2 spaces, single quotes
- Testing: vitest, *.test.ts

파일: /path/to/project/code-convention.md
```

### 부분 감지 시

```
✓ CLAUDE.md 업데이트 완료

저장된 커맨드:
- Build: `pnpm build`
- Test: `pnpm test`
- Lint: (감지 실패 - 수동 설정 필요)

⚠ Lint 커맨드를 감지하지 못했습니다.
필요하면 CLAUDE.md에 직접 추가해주세요.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 설정 파일 없음 | 사용자에게 직접 입력 요청 (AskUserQuestion) |
| 여러 설정 파일 존재 | 우선순위에 따라 선택하고 사용자에게 확인 |
| CLAUDE.md 읽기 실패 | 새 파일 생성 |
| CLAUDE.md 쓰기 실패 | 에러 메시지 출력, 수동 저장 안내 |
| 커맨드 일부만 감지 | 감지된 것만 저장, 나머지는 사용자에게 안내 |
| 잘못된 커맨드 형식 | 사용자에게 재입력 요청 |
| 소스 파일 없음 | 컨벤션 분석 건너뛰기, 사용자에게 안내 |
| 컨벤션 패턴 혼재 | AskUserQuestion으로 사용자에게 확인 |
| code-convention.md 쓰기 실패 | 에러 메시지 출력, 수동 저장 안내 |

### 설정 파일 우선순위

여러 설정 파일이 존재할 때 다음 순서로 우선합니다:

1. `package.json` (scripts 섹션이 있는 경우)
2. `Makefile`
3. `pyproject.toml`
4. `Cargo.toml`
5. `go.mod`
6. `build.gradle` / `pom.xml`

### 커맨드 검증

감지된 커맨드가 실제로 실행 가능한지 확인합니다:

```bash
# 커맨드 실행 가능 여부 확인 (dry-run)
command -v {package_manager} > /dev/null 2>&1
```

검증 실패 시 경고를 출력하고 사용자에게 확인을 요청합니다.

## Related Skills

- `/convention`: 이미 생성된 code-convention.md을 재분석하거나 수동 수정할 때 사용

## 지원하는 프로젝트 타입

### Node.js (package.json)

```json
{
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "lint": "eslint ."
  }
}
```

감지 규칙:
- `build`: scripts.build 또는 scripts.compile
- `test`: scripts.test
- `lint`: scripts.lint 또는 scripts.eslint

### Python (pyproject.toml)

```toml
[tool.pytest]
testpaths = ["tests"]

[tool.ruff]
line-length = 88
```

감지 규칙:
- `build`: `python -m build` 또는 `pip install -e .`
- `test`: `pytest` (pytest 섹션 존재 시)
- `lint`: `ruff check .` (ruff 섹션 존재 시) 또는 `flake8`

### Rust (Cargo.toml)

기본 커맨드:
- `build`: `cargo build`
- `test`: `cargo test`
- `lint`: `cargo clippy`

### Go (go.mod)

기본 커맨드:
- `build`: `go build ./...`
- `test`: `go test ./...`
- `lint`: `golangci-lint run`

### Make (Makefile)

타겟 기반 감지:
- `build`: `make build` 또는 `make all`
- `test`: `make test`
- `lint`: `make lint` 또는 `make check`

## 사용자 입력 옵션

설정 파일을 찾을 수 없거나 사용자가 수동 설정을 원할 때:

```
AskUserQuestion:
  question: "빌드 커맨드를 입력해주세요"
  header: "Build"
  options:
    - label: "npm run build"
      description: "npm 프로젝트 기본"
    - label: "make build"
      description: "Makefile 기반"
    - label: "직접 입력"
      description: "커스텀 커맨드"
```
