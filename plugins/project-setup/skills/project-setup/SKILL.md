---
name: project-setup
description: |
  프로젝트의 빌드/테스트 커맨드를 파악하고 CLAUDE.md에 저장합니다.
  "/project-setup", "프로젝트 설정", "빌드 설정" 요청 시 사용됩니다.
---

# Project Setup Skill

프로젝트의 빌드/테스트 커맨드를 파악하고 CLAUDE.md에 저장합니다.

## Workflow

```
1. 프로젝트 루트 파일 확인 (LS)
2. 설정 파일 읽고 빌드 커맨드 추론
3. 사용자 확인 (AskUserQuestion)
4. CLAUDE.md 저장
```

## Phase 1: 프로젝트 파일 확인

프로젝트 루트의 파일 목록을 확인합니다.

```bash
ls -la
```

## Phase 2: 빌드 커맨드 추론

설정 파일들을 읽고 빌드/테스트/린트 커맨드를 추론합니다.

**읽어볼 파일 예시:**
- `package.json`, `Makefile`, `Cargo.toml`, `go.mod`, `pyproject.toml`
- `build.gradle`, `pom.xml`, `mix.exs`, `Gemfile`
- `CMakeLists.txt`, `meson.build`, `justfile`
- 기타 프로젝트 설정 파일

**설정 파일이 없는 경우:**
사용자에게 직접 질문합니다.

## Phase 3: 사용자 확인

추론한 커맨드를 사용자에게 보여주고 확인받습니다.

```
## Detected Commands

- **Build**: `pnpm build`
- **Test**: `pnpm test`
- **Lint**: `pnpm lint`

이 설정을 CLAUDE.md에 저장할까요?
```

## Phase 4: CLAUDE.md 저장

확인된 커맨드를 CLAUDE.md에 저장합니다.

**출력 형식:**
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
2. "Build and Test Commands" 섹션이 있으면 내용 교체
3. 없으면 파일 시작 부분에 추가
4. 기존 내용은 보존

## Output

1. **CLAUDE.md 업데이트**: Build and Test Commands 섹션 추가
2. **결과 요약**: 저장된 커맨드 목록
