---
name: project-setup
description: |
  프로젝트/모듈 CLAUDE.md에 Convention 섹션(Project Convention, Code Convention)을 추가합니다.
  기존 프로젝트는 소스코드에서 컨벤션을 추출하고, 새 프로젝트는 대화형으로 수집합니다.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Write, AskUserQuestion]
---

# /project-setup

프로젝트 CLAUDE.md에 Convention 섹션을 추가하여 `/compile` REFACTOR 단계에서 참조할 수 있도록 합니다.

## Triggers

- `/project-setup`
- `프로젝트 설정`
- `컨벤션 생성`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | 자동 탐지 | 프로젝트 루트 경로 |

## Workflow

### 1. 프로젝트 루트 결정

인자가 있으면 해당 경로를 사용합니다. 없으면 자동 탐지합니다:

```bash
# .git, package.json, pyproject.toml, Cargo.toml, go.mod 등으로 탐지
find_project_root()
```

### 2. 모듈 루트 탐지

다음 build marker 파일을 기반으로 모듈 루트를 자동 감지합니다: `package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`, `build.gradle`, `build.gradle.kts`, `pom.xml`, `setup.py`, `CMakeLists.txt`.

프로젝트 루트에서 이 marker 파일들이 있는 디렉토리를 모듈 루트로 식별합니다. 모듈 루트가 발견되지 않으면 프로젝트 루트를 싱글 모듈로 취급합니다 (`project_root == module_root`).

### 3. 기존 Convention 섹션 확인

project_root CLAUDE.md와 각 module_root CLAUDE.md에서 Convention 섹션 존재 여부를 확인합니다:

`{project_root}/CLAUDE.md`를 Read하고, `## Project Convention` 섹션과 `## Code Convention` 섹션이 존재하는지 확인합니다.

기존 섹션이 존재하면 AskUserQuestion으로 처리 방법을 질문합니다:
- **덮어쓰기**: 기존 섹션을 새로 생성된 내용으로 대체
- **병합**: 기존 내용을 유지하면서 새 내용을 추가
- **취소**: 설정을 중단

### 4. 프로젝트 유형 판별

소스 파일 존재 여부로 기존 프로젝트와 신규 프로젝트를 구분합니다:

소스 파일(`*.ts`, `*.js`, `*.py`, `*.java`, `*.go`, `*.rs`, `*.kt`)을 Glob으로 검색하여 기존 프로젝트인지 판별합니다. 소스 파일이 하나라도 있으면 기존 프로젝트입니다.

### 5. 컨벤션 추출 또는 수집

#### 5-A. 기존 프로젝트: 코드 분석으로 추출

다음 항목을 자동 분석합니다:

| 분석 대상 | 방법 |
|-----------|------|
| 언어/런타임 | 파일 확장자 통계, `package.json`, `pyproject.toml` 등 |
| 디렉토리 패턴 | 최상위 디렉토리 구조 분석 |
| 코드 스타일 | 들여쓰기(탭/스페이스), 줄 길이, 세미콜론 사용 등 |
| 네이밍 규칙 | 변수/함수/클래스/상수의 네이밍 패턴 분석 |
| import 패턴 | import 순서, 그루핑 규칙 |
| 포맷터/린터 설정 | `.prettierrc`, `.eslintrc`, `ruff.toml` 등 |

분석 결과를 사용자에게 보여주고 AskUserQuestion으로 확인을 받습니다:
- **맞음**: 추출된 컨벤션을 그대로 사용
- **수정 필요**: 일부 항목을 수정하고 싶음

#### 5-B. 신규 프로젝트: 대화형 수집

AskUserQuestion 시퀀스로 컨벤션 정보를 수집합니다:

**Q1. 언어 선택**: "주요 프로그래밍 언어를 선택해주세요." 옵션: TypeScript(Node.js/Deno), Python(3.x), Go(1.x), Java/Kotlin(JVM)

**Q2. 구조 스타일**: "프로젝트 구조 스타일을 선택해주세요." 옵션: Layered(계층형 controller/service/repository), Feature-based(기능별 모듈 분리), Domain-driven(도메인 중심 패키지)

**Q3. 코딩 스타일**: "코딩 스타일 기본 규칙을 선택해주세요." 옵션: Strict(엄격한 타입/린트), Moderate(일반적 규칙), Minimal(최소한의 규칙)

### 6. project_root CLAUDE.md에 `## Project Convention` 섹션 추가

CLAUDE.md가 없으면 생성합니다. 있으면 끝에 섹션을 append합니다.

필수 3개 서브섹션은 반드시 포함합니다:

```markdown
## Project Convention

### Project Structure
(필수) 디렉토리 구조 규칙, 레이어링 패턴

### Module Boundaries
(필수) 모듈 책임 규칙, 의존성 방향

### Naming Conventions
(필수) 모듈/디렉토리/패키지 네이밍 규칙
```

선택 서브섹션 (분석/수집 결과에 따라 추가):

| 서브섹션 | 필수 | 설명 |
|----------|------|------|
| Project Structure | Yes | 디렉토리 구조 규칙, 레이어링 패턴 |
| Module Boundaries | Yes | 모듈 책임 규칙, 의존성 방향 |
| Naming Conventions | Yes | 모듈/디렉토리/패키지 네이밍 |
| API Design | No | REST/RPC 컨벤션 |
| Error Strategy | No | 글로벌 에러 핸들링 전략 |
| Testing Strategy | No | 테스트 조직, 커버리지 기대 |

### 7. 각 module_root CLAUDE.md에 `## Code Convention` 섹션 추가

싱글 모듈인 경우 project_root CLAUDE.md에 함께 추가합니다.
멀티 모듈인 경우 각 module_root CLAUDE.md에 추가합니다.

필수 3개 서브섹션은 반드시 포함합니다:

```markdown
## Code Convention

### Language & Runtime
(필수) 주요 언어, 버전, 런타임

### Code Style
(필수) 포맷팅, 들여쓰기, 줄 길이

### Naming Rules
(필수) 변수/함수/클래스/상수 네이밍 규칙
```

선택 서브섹션:

| 서브섹션 | 필수 | 설명 |
|----------|------|------|
| Language & Runtime | Yes | 주요 언어, 버전, 런타임 |
| Code Style | Yes | 포맷팅, 들여쓰기, 줄 길이 |
| Naming Rules | Yes | 변수/함수/클래스/상수 네이밍 |
| Type System | No | 타입 어노테이션 규칙 |
| Error Handling | No | try/catch 패턴, 에러 타입 |
| Import/Export | No | import 순서, barrel file 규칙 |
| Comments & Documentation | No | 주석/문서화 규칙 |

### 8. CLI 빌드 확인

`${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh`를 실행하여 CLI 바이너리를 확보합니다:

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
```

스크립트가 실패하면 AskUserQuestion으로 사용자에게 질문합니다:
- **설치**: rustup.rs 안내에 따라 Rust/Cargo를 설치 후 재시도
- **건너뛰기**: 검증 단계를 건너뜁니다

**"설치" 선택 시:**
1. Rust 설치 안내 표시:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   설치 후: source $HOME/.cargo/env
   ```
2. AskUserQuestion으로 설치 완료 여부를 확인합니다:
   - **완료**: 설치를 완료했습니다. CLI 빌드를 진행합니다.
   - **취소**: 설치를 건너뛰고 검증 단계를 스킵합니다.
3. "완료" 선택 시 `install-cli.sh` 재실행
4. 재실행에도 실패하면 검증 단계를 스킵

**"건너뛰기" 선택 시:** Step 9(검증)를 스킵하고 Step 10(결과 보고)으로 진행합니다.

### 9. 검증

`claude-md-core validate-convention` CLI를 실행하여 생성된 섹션을 검증합니다:

```bash
$CLI_PATH validate-convention --project-root {project_root}
```

### 10. 결과 보고

생성 완료 후 다음 내용을 사용자에게 안내합니다:

```
Convention 섹션이 CLAUDE.md에 추가되었습니다.

변경된 파일:
  - {project_root}/CLAUDE.md (## Project Convention 추가)
  - {module_root}/CLAUDE.md (## Code Convention 추가)

이 섹션들은 `/compile` 실행 시 REFACTOR 단계에서 자동 참조됩니다.
컨벤션을 수정하려면 `/convention-update`를 사용하세요.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 프로젝트 루트 탐지 실패 | 사용자에게 경로 입력 요청 |
| 파일 쓰기 권한 없음 | 에러 메시지 출력 |
| 소스 분석 실패 | 대화형 수집으로 전환 |
| install-cli.sh 실패 (cargo 미설치) | 사용자에게 설치/건너뛰기 질문, 건너뛰기 시 검증 스킵 |
| install-cli.sh 실패 (빌드 에러) | 에러 메시지 출력, 빌드 로그 확인 안내 |
| validate-convention 실패 | 에러 내용 표시, 수동 수정 안내 |
