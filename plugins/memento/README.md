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

기본 설치에는 Python 3.9+, Rust/Cargo, `uv`가 필요합니다. 플러그인을 설치한 다음, **설치된 플러그인 디렉터리**에서 한 번 실행합니다. 실행기는 플러그인 업데이트 후 다시 빌드합니다.

```sh
python3 scripts/install_runtime.py
```

마켓플레이스 체크아웃을 직접 사용하는 경우:

```sh
python3 plugins/memento/scripts/install_runtime.py
python3 plugins/memento/skills/memento/scripts/memento.py help
```

설치기는 `core/Cargo.toml`의 잠긴 의존성으로 release 실행기를 빌드하고 확인한 뒤 `skills/memento/bin/memento`에 배치합니다. Cargo는 캐시에 없는 의존성을 다운로드할 수 있습니다. 이미 빌드한 실행기가 있으면 `--binary /absolute/memento`를 사용할 수 있습니다. 배포 파일에는 특정 OS용 실행 파일이나 모델 가중치를 포함하지 않습니다.

기본 임베딩 모델은 `intfloat/multilingual-e5-small`입니다. 같은 설치 명령이 Python 3.12 환경·고정 의존성·모델 가중치를 준비하고 로컬 추론까지 확인합니다. 실행기와 설정은 모든 확인이 성공한 뒤 함께 반영하며 기존 사용자 파일을 보존합니다. 런타임·모델·hash로 식별한 worker는 플러그인 상위 디렉터리의 `.memento-semantic`에 보관해 같은 설정을 다시 설치할 때 재사용합니다. 같은 머신에서 플러그인 cache 경로를 옮겨도 그 외부 런타임을 유지하면 의미 검색 설정을 사용할 수 있습니다.

```sh
# 다른 임베딩 모델 선택
python3 scripts/install_runtime.py --embedding-model minilm
# 모델 설치 생략: 기존 모델 설정이 있으면 보존
python3 scripts/install_runtime.py --embedding-model none
```

`skills/memento/scripts/memento.py`로 조회하면 설치된 의미 검색 설정을 자동으로 사용합니다. 검색 JSON에서 `mode: semantic`을 선택하면 됩니다. `--semantic-config`를 명시하면 그 설정을 우선 사용합니다. 설치·업데이트에서 옵션을 생략하면 기본 E5 모델을 선택합니다.

일반 조회는 빌드나 다운로드를 실행하지 않습니다. 스킬은 DB 경로와 project/work/session 식별자를 명시해 중요한 작업 전환을 기록합니다. 대화 파일의 자동 검색·상주 수집기는 없으며 Git hook과 의미 검색 재정렬 모델은 선택적으로 설정합니다. Codex lifecycle hook은 아래 절차로 별도 신뢰·설정합니다. 상세 입력과 조회 계약은 [interface](skills/memento/references/interface.md), 의미 검색 설정은 [semantic](skills/memento/references/semantic.md)를 참조하세요.

## Codex 체크포인트 훅

Memento 0.4.0은 Codex가 스킬 설명을 통해 기록·조회 기능을 선택하도록 하고, trusted command hook으로 이벤트와 실제 저장 여부를 검사합니다. 자동 선택과 자연어 중요 내용의 완전성을 보장하지는 않습니다.

1. 플러그인을 설치·활성화하고 설치된 패키지의 `scripts/install_runtime.py`를 실행합니다.
2. Codex의 hook review UI(CLI `/hooks`)에서 현재 정의를 검토하고 신뢰합니다. 설치기는 이 신뢰 설정을 변경하지 않습니다.
3. 시작 훅이 안내한 실제 `PLUGIN_DATA` 경로에 프로젝트를 명시적으로 설정합니다.

```sh
python3 /absolute/installed-plugin/codex-hooks/capture.py configure \
  --data-dir /absolute/plugin-data --repository /absolute/git-root \
  --store /absolute/context.sqlite --project-id upload-app --work-id upload-429 \
  --initialize-store
```

store는 기존 저장소를 선택할 수 있으며 `--initialize-store`는 선택한 journal source 초기화에만 필요합니다. 사용자 literal masking 정책이 있으면 같은 `--policy /absolute/policy.json`을 사용합니다.

시작·요청 훅은 스킬 경로와 scope를 안내하고, 인지된 첫 변경 전에 미해결 사용자 요청을 검사합니다. 변경·검증·실패 관측은 모아서 의미 레코드로 기록한 뒤 exact source/record/revision/sequence로 의무를 해결합니다. `Stop`은 알려진 미완료 의무에 최대 두 번 보충 기회를 주며 지속 실패는 `capture_incomplete`로 고지합니다. 일반 편집마다 별도 의미 기록을 강제하지 않습니다. 저장 의무는 용량 제한 안에서 관리하고 정확한 완료 레코드를 Datalog compaction에서 보호합니다.

선택적 Git gate는 `hooks-install --enforce-checkpoints true`로 켭니다. `prepare_commit`과 새 의미 기록을 연결한 뒤 실제 parent HEAD·index tree가 맞아야 커밋할 수 있습니다. 부분 커밋 임시 index와 worktree도 구분합니다. Post-commit은 관측 가능한 직전 HEAD가 있으면 실제 SHA를 연결합니다. 훅을 우회하는 Git 경로와 임의 shell 프로그램·MCP 도구의 전체 변경은 강제 범위 밖입니다.

자세한 기록·영수증·Stop·압축 계약은 [Codex hook 사용법](skills/memento/references/codex-hooks.md)을 참조하세요. **이 hook은 Codex 전용입니다.** Claude Code는 기존 스킬·CLI·선택적 Git hook을 사용하며 Codex의 lifecycle enforcement를 제공한다고 표시하지 않습니다.

## 구현 출처와 검증

Rust core·시나리오 테스트·스킬은 [0pg-mcp의 Memento 구현](https://github.com/0pg/0pg-mcp/tree/8d151993/crates/memento)에서 가져왔습니다. 이 패키지는 원본 저장소 없이 빌드·실행할 수 있습니다. 스킬 경로와 설치 안내는 플러그인 구조에 맞췄으며 원본 Codex 설치기 전용 테스트는 플러그인 실행기 설치 테스트로 대체했습니다.

```sh
cargo fmt --manifest-path plugins/memento/core/Cargo.toml --check
cargo clippy --manifest-path plugins/memento/core/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path plugins/memento/core/Cargo.toml --all-features
python3 -m unittest discover -s plugins/memento/tests
```

배포 메타데이터: Claude Code는 `.claude-plugin/plugin.json`, Codex는 root `plugin.json`의 `extensions.com.openai`와 저장소의 `.agents/plugins/marketplace.json`을 사용합니다. `.codex-plugin/plugin.json`은 동일한 호환 메타데이터를 유지합니다. Codex 훅은 `codex-hooks/hooks.json`으로 선언해 Claude의 기본 `hooks/hooks.json` 자동 로드와 분리했습니다. [Claude Code 배포 문서](https://code.claude.com/docs/en/plugin-marketplaces), [Claude hook 로드 규칙](https://code.claude.com/docs/en/plugins-reference#hooks), [OpenAI 플러그인 패키징 문서](https://developers.openai.com/plugins/build/plugins)를 참조하세요.
