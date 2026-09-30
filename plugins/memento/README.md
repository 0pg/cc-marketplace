# Memento

Claude Code와 Codex에서 작업의 요청·결정·실패한 시도·정정·검증·Git 체크포인트를 기록하고 근거를 조회하는 플러그인입니다. 두 제품은 같은 스킬과 Rust 실행기를 사용합니다. 기록은 로컬 SQLite에 저장하며 Crepe Datalog 규칙으로 보존 범위를 결정합니다.

## Claude Code 설치

```sh
claude plugin marketplace add 0pg/cc-marketplace
claude plugin install memento@jhk-plugins
```

설치 후 `/memento:memento`를 사용합니다.

## Codex 설치

```sh
codex plugin marketplace add 0pg/cc-marketplace
codex plugin add memento@jhk-plugins
```

설치 후 새 세션에서 `$memento:memento`를 사용합니다. Codex 데스크톱 앱에서도 등록된 `jhk Plugins` 마켓플레이스의 Memento를 설치할 수 있습니다. 로컬 체크아웃을 사용하려면 `codex plugin marketplace add /absolute/cc-marketplace`로 등록합니다.

## 실행기 준비

Python 3.9+와 Rust/Cargo가 필요합니다. 플러그인을 설치한 다음, **설치된 플러그인 디렉터리**에서 한 번 실행합니다. 실행기는 플러그인 업데이트 후 다시 빌드합니다.

```sh
python3 scripts/install_runtime.py
```

마켓플레이스 체크아웃을 직접 사용하는 경우:

```sh
python3 plugins/memento/scripts/install_runtime.py
python3 plugins/memento/skills/memento/scripts/work-context.py help
```

설치기는 `core/Cargo.toml`의 잠긴 의존성으로 release 실행기를 빌드하고 확인한 뒤 `skills/memento/bin/work-context`에 배치합니다. Cargo는 캐시에 없는 의존성을 다운로드할 수 있습니다. 이미 빌드한 실행기가 있으면 `--binary /absolute/work-context`를 사용할 수 있습니다. 배포 파일에는 특정 OS용 실행 파일을 포함하지 않습니다.

일반 조회는 빌드나 다운로드를 실행하지 않습니다. 스킬은 DB 경로와 project/work/session 식별자를 명시해 중요한 작업 전환을 기록합니다. 대화 파일의 자동 검색·상주 수집기는 없으며 Git hook과 로컬 의미 검색 모델은 선택적으로 설정합니다. 상세 입력과 조회 계약은 [interface](skills/memento/references/interface.md), 선택적 모델 설정은 [semantic](skills/memento/references/semantic.md)를 참조하세요.

## 구현 출처와 검증

Rust core·시나리오 테스트·스킬은 [0pg-mcp의 Memento 구현](https://github.com/0pg/0pg-mcp/tree/677b8800b40253ce44f6cf2ef03de38f25cf28d4/crates/work-context)에서 가져왔습니다. 이 패키지는 원본 저장소 없이 빌드·실행할 수 있습니다. 스킬 경로와 설치 안내는 플러그인 구조에 맞췄으며 원본 Codex 설치기 전용 테스트는 플러그인 실행기 설치 테스트로 대체했습니다.

```sh
cargo fmt --manifest-path plugins/memento/core/Cargo.toml --check
cargo clippy --manifest-path plugins/memento/core/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path plugins/memento/core/Cargo.toml --all-features
python3 -m unittest discover -s plugins/memento/tests
```

배포 메타데이터: Claude Code는 `.claude-plugin/plugin.json`, Codex는 `.codex-plugin/plugin.json`과 저장소의 `.agents/plugins/marketplace.json`을 사용합니다. [Claude Code 배포 문서](https://code.claude.com/docs/en/plugin-marketplaces)와 [OpenAI 플러그인 패키징 문서](https://developers.openai.com/plugins/build/plugins)의 로컬 마켓플레이스 형식에 맞췄습니다.
