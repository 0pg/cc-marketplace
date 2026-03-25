---
name: project-init
description: |
  Multi-language project setup plugin. Sets up code conventions, dependencies,
  CLAUDE.md, and installs a language-specific convention skill into the target project.
  "/project-init", "프로젝트 생성", "project init", "프로젝트 초기화", "project setup" 요청 시 사용됩니다.
allowed-tools: [Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion]
argument-hint: "[--lang rust] [--name <name>] [--type cli,backend,frontend] [--db toasty|none|<custom>]"
---

# Project Init Command

신규 또는 기존 프로젝트에 코드 컨벤션, 의존성, CLAUDE.md, convention skill을 적용합니다.

---

## Phase 0: Language Selection

### Direct 모드
`--lang` 인자가 주어진 경우:
- `references/` 하위에 해당 언어 디렉토리가 존재하는지 확인
- 존재하지 않으면 에러 메시지 출력 후 종료

### Interactive 모드
`--lang` 인자가 없으면 `AskUserQuestion`:
```
프로젝트 언어를 선택하세요:
- rust

현재 지원 언어: rust
```

파싱 결과: `LANG` (예: `rust`)

---

## Phase 1: Argument Parsing & Interactive Flow

### Direct 모드
인자가 주어진 경우 파싱:
- `--name <name>`: 프로젝트 이름
- `--type <types>`: 쉼표 구분 복수 선택 (언어별 옵션)
- `--db <driver>`: DB 드라이버 (언어별 옵션)

### Interactive 모드
인자가 없으면 순차적으로 질문. 질문 항목은 언어별로 다름.

### Rust (LANG == rust)

1. **프로젝트 이름**: `AskUserQuestion`. 기본값은 현재 디렉토리명.
2. **프로젝트 타입**: `AskUserQuestion`으로 복수 선택.
   ```
   프로젝트 타입을 선택하세요 (복수 선택 가능, 쉼표 구분):
   - cli: CLI 앱 (clap)
   - backend: 백엔드 서버 (tokio + axum)
   - frontend: 프론트엔드 (leptos)

   예: cli,backend
   ```
3. **DB 드라이버**: `AskUserQuestion`.
   ```
   DB 드라이버를 설정할까요?
   - toasty (AWS Labs ORM)
   - 다른 크레이트명 직접 입력
   - none (DB 불필요)

   기본값: none
   ```

파싱 결과:
- `PROJECT_NAME`: 프로젝트 이름
- `PROJECT_TYPES`: 선택된 타입 리스트
- `DB_DRIVER`: DB 드라이버 (none이면 생략)

---

## Phase 2: VCS + Project Init (Idempotent)

1. `.git/` 디렉토리 존재 여부 확인
   - 없으면: `git init` 실행
   - 있으면: skip

### Rust (LANG == rust)
2. `Cargo.toml` 존재 여부 확인
   - 없으면: `cargo init --name <PROJECT_NAME>` 실행
   - 있으면: skip

---

## Phase 3: Formatter Config

### Rust (LANG == rust)
1. `references/rust/rustfmt.toml`을 Read
2. 프로젝트 루트에 `rustfmt.toml`로 Write (이미 존재하면 덮어쓰기 — convention이 source of truth)

---

## Phase 4: Lints

### Rust (LANG == rust)
1. 프로젝트 루트의 `Cargo.toml`을 Read
2. `[package]` 섹션에서 `edition` 값을 `"2024"`로 설정 (Edit)
3. `references/rust/cargo-lints.toml`을 Read
4. `[lints.clippy]`와 `[lints.rust]` 섹션을 Cargo.toml에 적용 (Edit)
   - `[lints` 섹션이 없으면: Cargo.toml 끝에 추가
   - `[lints` 섹션이 있으면: **merge (upsert)** — reference의 각 키를 추가하거나 기존 값을 갱신하되, 사용자가 추가한 다른 lint 키는 보존

---

## Phase 5: Dependencies

### Rust (LANG == rust)

`cargo add`를 사용하여 의존성을 설치합니다. 버전은 cargo가 자동으로 최신 호환 버전을 해석합니다.

#### 5-1. 공통 의존성 (항상 설치)
1. `references/rust/deps-common.toml`을 Read
2. 각 크레이트에 대해 `cargo add <name>` 실행 (features가 있으면 `--features` 플래그 추가)

#### 5-2. 타입별 의존성
선택된 `PROJECT_TYPES`에 해당하는 reference 파일만 처리:
- **CLI**: `references/rust/deps-cli.toml`을 Read → `cargo add`
- **Backend**: `references/rust/deps-backend.toml`을 Read → `cargo add`
- **Frontend**: `references/rust/deps-frontend.toml`을 Read → `cargo add`

#### 5-3. DB 드라이버 (선택된 경우)
- `DB_DRIVER`가 `none`이 아니면: `cargo add <DB_DRIVER>`

### 오류 처리
- `cargo add` 실패 시 오류 메시지를 출력하되 다음 의존성 설치를 계속 진행
- 모든 설치 완료 후 실패 목록이 있으면 사용자에게 알림

---

## Phase 6: CLAUDE.md Generation

1. `references/{LANG}/claude-md-template.md`를 Read
2. 플레이스홀더 치환:
   - `{{PROJECT_NAME}}` → `PROJECT_NAME`
   - `{{PROJECT_TYPES}}` → 선택된 타입 (예: "cli, backend")
3. 프로젝트 루트에 `CLAUDE.md`로 Write
   - 이미 존재하면 `AskUserQuestion`으로 덮어쓰기 여부 확인
   - 사용자가 거부하면 skip

---

## Phase 7: Convention Skill Installation

언어별 convention skill을 대상 프로젝트의 `.claude/skills/`에 설치합니다.

1. `references/{LANG}/convention/SKILL.md`를 Read
2. 대상 경로: `{PROJECT_ROOT}/.claude/skills/{LANG}-convention/SKILL.md`
3. `.claude/skills/{LANG}-convention/` 디렉토리가 없으면 생성
4. 대상 파일이 이미 존재하면:
   - 내용이 동일 → skip (이미 최신)
   - 내용이 다름 → `AskUserQuestion`으로 덮어쓰기 여부 확인 (기존 커스터마이징 유실 가능성 안내)
5. 존재하지 않으면 Write

---

## Phase 8: Superpowers Plugin Installation

Claude Code superpowers 플러그인을 설치합니다. 언어와 무관하게 항상 실행.

1. `.claude/plugins` 또는 설정에서 superpowers 존재 여부 확인 (Grep)
2. 이미 설치된 경우 skip
3. 설치되지 않은 경우, Claude Code 내부 커맨드이므로 사용자에게 직접 실행하도록 안내:
   ```
   superpowers 플러그인을 설치하려면 다음 명령을 실행하세요:
   1. /plugin marketplace add obra/superpowers-marketplace
   2. /plugin install superpowers@superpowers-marketplace
   ```

---

## Phase 9: Summary

모든 Phase 완료 후 결과를 요약합니다:

```markdown
## Project Init 완료

### 프로젝트 정보
- **이름**: {PROJECT_NAME}
- **언어**: {LANG}
- **타입**: {PROJECT_TYPES}

### 생성/수정된 파일
- (언어별 설정 파일 목록)
- `CLAUDE.md` — 프로젝트 컨벤션 문서
- `.claude/skills/{LANG}-convention/SKILL.md` — 코드 컨벤션 스킬

### 다음 단계
(언어별 빌드/테스트/lint 명령)
```

---

## DO / DON'T

### DO
- 각 Phase에서 파일 존재 여부를 먼저 확인 (idempotent)
- reference 파일을 Read하여 템플릿 내용 확인
- convention skill 설치 시 기존 파일 존재 여부 확인
- 언어별 패키지 매니저로 의존성 설치 (설정 파일 직접 편집 X)

### DON'T
- 사용자가 거부한 파일을 덮어쓰지 않음
- 의존성 설치 실패 시 전체 프로세스를 abort하지 않음
- 지원하지 않는 언어로 진행하지 않음
