---
name: rust-init
description: |
  Rust 프로젝트 셋업 플러그인. 신규 또는 기존 프로젝트에 cargo init, 코드 컨벤션(rustfmt, clippy lints),
  의존성 템플릿, CLAUDE.md를 단일 커맨드로 적용합니다.
  "/rust-init", "rust 프로젝트 생성", "rust init", "러스트 초기화", "rust setup" 요청 시 사용됩니다.
user_invocable: true
allowed-tools: [Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion]
argument-hint: "[--name <name>] [--type cli,backend,frontend] [--db toasty|none|<custom>]"
---

# Rust Init Skill

신규 또는 기존 Rust 프로젝트에 코드 컨벤션, 의존성, CLAUDE.md를 적용합니다.

---

## Phase 0: Argument Parsing & Interactive Flow

### Direct 모드
인자가 주어진 경우 파싱:
- `--name <name>`: 프로젝트 이름
- `--type <types>`: 쉼표 구분 복수 선택 (cli, backend, frontend)
- `--db <driver>`: DB 드라이버 (toasty / none / 커스텀 크레이트명)

### Interactive 모드
인자가 없으면 순차적으로 질문:

1. **프로젝트 이름**: `AskUserQuestion`으로 질문. 기본값은 현재 디렉토리명.
2. **프로젝트 타입**: `AskUserQuestion`으로 복수 선택 가능하게 질문.
   ```
   프로젝트 타입을 선택하세요 (복수 선택 가능, 쉼표 구분):
   - cli: CLI 앱 (clap)
   - backend: 백엔드 서버 (tokio + axum)
   - frontend: 프론트엔드 (leptos)

   예: cli,backend
   ```
3. **DB 드라이버**: `AskUserQuestion`으로 질문.
   ```
   DB 드라이버를 설정할까요?
   - toasty (AWS Labs ORM)
   - 다른 크레이트명 직접 입력
   - none (DB 불필요)

   기본값: none
   ```

파싱 결과를 변수로 정리:
- `PROJECT_NAME`: 프로젝트 이름
- `PROJECT_TYPES`: 선택된 타입 리스트
- `DB_DRIVER`: DB 드라이버 (none이면 생략)

---

## Phase 1: Git + Cargo Init (Idempotent)

1. `.git/` 디렉토리 존재 여부 확인 (`ls .git/`)
   - 없으면: `git init` 실행
   - 있으면: skip
2. `Cargo.toml` 존재 여부 확인 (`ls Cargo.toml`)
   - 없으면: `cargo init --name <PROJECT_NAME>` 실행
   - 있으면: skip, 다음 Phase로 진행

---

## Phase 2: rustfmt.toml

1. `references/rustfmt.toml` 파일을 Read
2. 프로젝트 루트에 `rustfmt.toml`로 Write (이미 존재하면 덮어쓰기 — convention이 source of truth)

---

## Phase 3: Cargo.toml — Edition + Lints

1. 프로젝트 루트의 `Cargo.toml`을 Read
2. `[package]` 섹션에서 `edition` 값을 `"2024"`로 설정 (Edit)
3. `references/cargo-lints.toml`을 Read
4. `[lints.clippy]`와 `[lints.rust]` 섹션을 Cargo.toml에 적용 (Edit)
   - `[lints` 섹션이 없으면: Cargo.toml 끝에 추가
   - `[lints` 섹션이 있으면: **merge (upsert)** — reference의 각 키를 추가하거나 기존 값을 갱신하되, 사용자가 추가한 다른 lint 키는 보존

### 주입할 lint 규칙
```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
pedantic = "warn"

[lints.rust]
unsafe_code = "deny"
```

---

## Phase 4: Cargo.toml — Dependencies

`cargo add`를 사용하여 의존성을 설치합니다. 버전은 cargo가 자동으로 최신 호환 버전을 해석합니다.

### 4-1. 공통 의존성 (항상 설치)
1. `references/deps-common.toml`을 Read
2. 각 크레이트에 대해 `cargo add <name>` 실행 (features가 있으면 `--features` 플래그 추가)

### 4-2. 타입별 의존성
선택된 `PROJECT_TYPES`에 해당하는 reference 파일만 처리:
- **CLI**: `references/deps-cli.toml`을 Read → `cargo add`
- **Backend**: `references/deps-backend.toml`을 Read → `cargo add`
- **Frontend**: `references/deps-frontend.toml`을 Read → `cargo add`

### 4-3. DB 드라이버 (선택된 경우)
- `DB_DRIVER`가 `none`이 아니면: `cargo add <DB_DRIVER>`

### 오류 처리
- `cargo add` 실패 시 오류 메시지를 출력하되 다음 의존성 설치를 계속 진행
- 모든 설치 완료 후 실패 목록이 있으면 사용자에게 알림

---

## Phase 5: CLAUDE.md Generation

1. `references/claude-md-template.md`를 Read
2. 플레이스홀더 치환:
   - `{{PROJECT_NAME}}` → `PROJECT_NAME`
   - `{{PROJECT_TYPES}}` → 선택된 타입 (예: "cli, backend")
3. 프로젝트 루트에 `CLAUDE.md`로 Write
   - 이미 존재하면 `AskUserQuestion`으로 덮어쓰기 여부 확인
   - 사용자가 거부하면 skip

---

## Phase 6: Summary

모든 Phase 완료 후 결과를 요약합니다:

```markdown
## Rust Init 완료

### 프로젝트 정보
- **이름**: {PROJECT_NAME}
- **타입**: {PROJECT_TYPES}
- **Edition**: 2024

### 생성/수정된 파일
- `Cargo.toml` — edition, lints, dependencies 설정
- `rustfmt.toml` — 코드 포맷팅 규칙
- `CLAUDE.md` — 프로젝트 컨벤션 문서

### 설치된 의존성
- 공통: tracing, serde, serde_json, thiserror, anyhow
- {타입별 의존성 목록}

### 다음 단계
1. `cargo build` — 빌드 확인
2. `cargo test` — 테스트 실행
3. `cargo clippy` — lint 확인
```

---

## DO / DON'T

### DO
- 각 Phase에서 파일 존재 여부를 먼저 확인 (idempotent)
- `cargo add`로 의존성 설치 (TOML 직접 편집 X)
- reference 파일을 Read하여 템플릿 내용 확인

### DON'T
- Cargo.toml의 `[dependencies]`를 직접 편집하지 않음
- 사용자가 거부한 파일을 덮어쓰지 않음
- `cargo add` 실패 시 전체 프로세스를 abort하지 않음
