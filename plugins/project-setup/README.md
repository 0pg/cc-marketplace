# Project Setup Plugin

프로젝트의 빌드/테스트 커맨드를 파악하고 CLAUDE.md에 저장합니다.

## Installation

```bash
claude plugins install ./plugins/project-setup
```

## Usage

```
/project-setup
```

## 동작 방식

1. 프로젝트 루트의 파일들을 확인
2. 설정 파일(package.json, Makefile, Cargo.toml 등)을 읽고 빌드 커맨드 추론
3. 사용자에게 확인 요청
4. CLAUDE.md에 저장

## 출력 예시

```markdown
# Build and Test Commands

- **Build**: `pnpm build`
- **Test**: `pnpm test`
- **Lint**: `pnpm lint`

# Workflow
- Run tests after making code changes
- Ensure linting passes before committing
```

## License

MIT
