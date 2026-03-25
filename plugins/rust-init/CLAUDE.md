# rust-init Plugin

## Purpose
Rust 프로젝트 초기화 플러그인. 단일 커맨드(`/rust-init`)로 cargo init, 코드 컨벤션(rustfmt, clippy lints),
의존성 설정, CLAUDE.md 생성, superpowers 플러그인 설치를 수행합니다.

## Directory Structure
```
plugins/rust-init/
├── .claude-plugin/plugin.json
├── CLAUDE.md
└── skills/rust-init/
    ├── SKILL.md
    └── references/
        ├── rustfmt.toml           # rustfmt 설정 템플릿
        ├── cargo-lints.toml       # [lints] 섹션 템플릿
        ├── deps-common.toml       # 공통 의존성
        ├── deps-cli.toml          # CLI 프로젝트 의존성
        ├── deps-backend.toml      # Backend 프로젝트 의존성
        ├── deps-frontend.toml     # Frontend 프로젝트 의존성
        └── claude-md-template.md  # CLAUDE.md 템플릿
```

## Workflow
1. 인자 파싱 (interactive vs direct mode)
2. cargo init (idempotent)
3. rustfmt.toml 생성
4. Cargo.toml에 lints + deps 병합
5. CLAUDE.md 생성
6. superpowers 플러그인 설치 안내
7. 결과 요약

## Convention
- Edition: 2024
- Panic-free: deny unwrap_used, expect_used, panic, todo, unimplemented, unreachable
- Module style: {module}.rs (not mod.rs)
- unsafe_code = "deny"
