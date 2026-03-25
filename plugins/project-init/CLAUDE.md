# project-init Plugin

## Purpose
Multi-language 프로젝트 셋업 플러그인. 단일 커맨드(`/project-init`)로 신규 또는 기존 프로젝트에
코드 컨벤션, 의존성, CLAUDE.md, convention skill을 적용합니다.
v1.0은 Rust를 지원하며, `references/{lang}/` 구조로 언어를 확장합니다.

## Directory Structure
```
plugins/project-init/
├── .claude-plugin/plugin.json
├── CLAUDE.md
└── commands/
    ├── project-init.md                    # /project-init command
    └── references/
        └── rust/                          # 언어별 디렉토리
            ├── convention/
            │   └── SKILL.md               # → 대상 프로젝트 .claude/skills/에 설치
            ├── rustfmt.toml
            ├── cargo-lints.toml
            ├── deps-common.toml
            ├── deps-cli.toml
            ├── deps-backend.toml
            ├── deps-frontend.toml
            └── claude-md-template.md
```

## Workflow
1. 언어 선택
2. 인자 파싱 (interactive vs direct mode)
3. VCS + project init (idempotent)
4. Formatter config
5. Lints merge (upsert)
6. Dependencies 추가
7. CLAUDE.md 생성
8. Convention skill → `.claude/skills/{lang}-convention/` 설치
9. Superpowers plugin 설치 안내
10. 결과 요약

## Extensibility
새 언어 추가 시:
1. `references/{lang}/` 디렉토리 생성
2. 해당 언어의 formatter, lints, deps, CLAUDE.md 템플릿, convention skill 추가
3. command의 각 Phase에 `### {Lang} (LANG == {lang})` 섹션 추가
