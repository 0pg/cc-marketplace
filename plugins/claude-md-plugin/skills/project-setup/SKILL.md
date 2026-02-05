---
name: project-setup
version: 1.0.0
aliases: [setup, build-setup]
trigger:
  - /project-setup
  - 프로젝트 설정
  - 빌드 설정
description: |
  This skill should be used when the user asks to "set up project", "configure build commands",
  "detect test commands", or uses "/project-setup".
  Auto-detects build/test/lint commands and persists them to CLAUDE.md.

  <example>
  <user_request>/project-setup</user_request>
  <assistant_response>
  프로젝트 파일을 분석합니다...

  ## Detected Commands

  - **Build**: `pnpm build`
  - **Test**: `pnpm test`
  - **Lint**: `pnpm lint`

  이 설정을 CLAUDE.md에 저장할까요?
  </assistant_response>
  </example>
allowed-tools: [Bash, Read, Glob, Write, AskUserQuestion]
---

# Project Setup Skill

## 목적

프로젝트의 빌드/테스트/린트 커맨드를 자동 감지하고 CLAUDE.md에 저장합니다.

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

### Phase 4: CLAUDE.md 저장

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

## 출력 형식

```
✓ CLAUDE.md 업데이트 완료

저장된 커맨드:
- Build: `pnpm build`
- Test: `pnpm test`
- Lint: `pnpm lint`

파일: /path/to/project/CLAUDE.md
```
