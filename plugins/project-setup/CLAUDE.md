# Project Setup Plugin

프로젝트의 빌드/테스트 커맨드를 파악하고 CLAUDE.md에 저장하는 플러그인입니다.

## Workflow

```
1. 프로젝트 루트 파일 확인
2. 설정 파일 읽고 빌드 커맨드 추론
3. 사용자 확인
4. CLAUDE.md 저장
```

## 출력 형식

```markdown
# Build and Test Commands

- **Build**: `pnpm build`
- **Test**: `pnpm test`
- **Lint**: `pnpm lint`

# Workflow
- Run tests after making code changes
- Ensure linting passes before committing
```
