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

## 자동 실행기 준비와 업데이트

Memento `0.6.0`은 core `0.3.0`, CLI protocol `1`, Store format `2`을 사용합니다. 설치 후 스킬의 `skills/memento/scripts/memento.py` launcher로 실제 기록·조회 명령을 실행하면 누락되거나 갱신이 필요한 runtime을 준비한 뒤 원래 명령을 이어서 실행합니다. Codex가 제공하는 선택적 `setup-memento` 설정 대화도 같은 준비 루틴을 사용합니다. 설치 자체가 post-install script를 실행하거나 hook 신뢰를 부여하는 것은 아닙니다.

소스 패키지에는 Python 3.9+, Rust/Cargo 1.94+, 기본 모델 준비에는 `uv`가 필요합니다. 설치된 플러그인 디렉터리에서 수동 준비와 상태 확인도 가능합니다.

```sh
python3 scripts/install_runtime.py --ensure
python3 scripts/install_runtime.py --status
python3 skills/memento/scripts/memento.py runtime-status
# 명시적인 모델 선택
python3 scripts/install_runtime.py --embedding-model minilm
# 모델 준비 생략: 이미 있는 의미 검색 설정은 유지
python3 scripts/install_runtime.py --embedding-model none
# 호환 실행기가 이미 있으면 Cargo 빌드 생략
python3 scripts/install_runtime.py --binary /absolute/memento --embedding-model none
```

알려진 새 설치는 `intfloat/multilingual-e5-small`을 기본으로 준비하고, 업데이트에서는 기존 E5·MiniLM·사용자 config 선택을 유지합니다. 구 설치에 실행기만 있고 선택 정보나 config가 없으면 `selection_required`로 중단하며 E5·MiniLM·`none` 중 명시한 선택으로 재개합니다. `none`은 기존 의미 검색 설정을 비활성화하지 않습니다. 공급된 모델은 Python 3.12·고정 의존성·가중치를 준비하고 실제 로컬 추론을 확인한 뒤 활성화합니다. Cargo·모델 다운로드·추론 실패는 단계별 오류로 드러나며 OS 도구는 자동 설치하지 않습니다.

Runtime은 `${CODEX_HOME:-~/.codex}/memento/runtime`, 또는 `MEMENTO_RUNTIME_HOME`에 둡니다. 플러그인 캐시가 교체되어도 실행기·model·config를 재사용합니다. 설치 잠금으로 동시 초기화를 조정하며 protocol·build identity·OS/architecture를 검사한 candidate만 활성화합니다. 관리 artifact는 active와 previous 각 1개, 진행 candidate 1개와 그 모델 참조로 제한하고 Memento 소유가 아닌 파일은 보존합니다. 실패한 업데이트는 기존 active와 DB를 유지하고 실패를 반환합니다.

`runtime-status`, installer `--status`, help, version은 설치를 시작하거나 DB를 열지 않습니다. `store-status`도 runtime 준비를 시작하지 않으며 사용할 수 있는 호환 실행기로 선택된 DB를 읽기만 합니다. 준비 전 source-only 패키지의 help/version은 실행기 부재를 알릴 수 있습니다. 준비 이후 일반 기록·조회는 같은 검증된 artifact를 사용하며 변경된 source/model이 있을 때만 다시 준비합니다. `query`는 active 의미 검색 config를 사용하고 명시적인 `--semantic-config`가 우선합니다.

## 기존 데이터와 버전 확인

```sh
python3 skills/memento/scripts/memento.py version
python3 skills/memento/scripts/memento.py store-status --store /absolute/context.sqlite
python3 skills/memento/scripts/memento.py migrate --store /absolute/context.sqlite
```

Runtime 준비는 DB를 검색하거나 변환하지 않습니다. `store-status`는 선택된 DB의 버전 없는 known legacy를 `migration_required`로 표시하고 파일을 변경하거나 생성하지 않습니다. 명시적 `migrate`와 실제 store를 여는 명령은 알려진 버전 없는 legacy에는 `store_format` header를 추가하고 format 1은 format 2로 한 SQLite transaction 안에서 갱신합니다. Format 1→2는 같은 DB identity를 유지하고 migration epoch를 증가시킵니다. Entity 행을 다시 삽입하지 않아 ID·revision·sequence·근거·compaction·capture 상태를 유지합니다. SQL schema나 원문을 변환하지 않으므로 SQLite transaction journal로 rollback하며 외부 plaintext backup은 남기지 않습니다.

Store별 빈 `.memento-lock` sidecar로 writer를 조정하고 transaction마다 identity/epoch를 검사합니다. 알 수 없는 schema/payload, 미래 형식, 용량 초과는 오류로 중단합니다. 실제 구버전 `0.3.0`·`0.4.0` 실행기는 새 metadata field를 거절하며, format 1 실행기도 format 2 저장소에 기록할 수 없습니다. Runtime 전환과 DB 복원은 별개이며 과거 snapshot으로 새 기록을 제거하는 자동 복원은 없습니다.

스킬은 DB 경로와 project/work/session 식별자를 명시해 중요한 작업 전환을 기록합니다. 대화 파일 자동 검색·상주 수집기는 없으며 Git hook과 의미 검색 재정렬 모델은 선택적으로 설정합니다. 상세 입력·조회·오류 계약은 [interface](skills/memento/references/interface.md), 의미 검색은 [semantic](skills/memento/references/semantic.md)를 참조하세요.

## Codex 체크포인트 훅

Memento 0.6.0은 Codex가 스킬 설명을 통해 기록·조회 기능을 선택하도록 하고, trusted command hook으로 이벤트와 실제 저장 여부를 검사합니다. 자동 선택과 자연어 중요 내용의 완전성을 보장하지는 않습니다.

1. 플러그인을 설치·활성화하고 스킬 launcher 또는 선택적 setup 흐름으로 runtime을 준비합니다. 수동으로는 설치된 패키지의 `scripts/install_runtime.py --ensure`를 실행합니다.
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

SessionStart는 준비된 실행기를 확인하거나 준비 안내를 제공하며 build/model download를 실행하지 않습니다. 새로 설치한 native Git hook은 cache/artifact 경로 대신 stable `<runtime-home>/git-memento` bridge를 사용합니다. 기존 hook은 `hooks-status`의 target 상태를 확인하세요. 이전 경로를 판독할 수 없는 managed hook은 `legacy_path_review_required`, 사라진 target은 `missing`으로 표시하므로 명시적으로 검토·재설치합니다. Runtime 준비가 Git gate를 켜거나 수정된 사용자 hook을 덮어쓰지는 않습니다.

선택적 Git gate는 `hooks-install --enforce-checkpoints true`로 켭니다. `prepare_commit`과 새 의미 기록을 연결한 뒤 실제 parent HEAD·index tree가 맞아야 커밋할 수 있습니다. 부분 커밋 임시 index와 worktree도 구분합니다. Post-commit은 관측 가능한 직전 HEAD가 있으면 실제 SHA를 연결합니다. 훅을 우회하는 Git 경로와 임의 shell 프로그램·MCP 도구의 전체 변경은 강제 범위 밖입니다.

자세한 기록·영수증·Stop·압축 계약은 [Codex hook 사용법](skills/memento/references/codex-hooks.md)을 참조하세요. **이 hook은 Codex 전용입니다.** Claude Code는 기존 스킬·CLI·선택적 Git hook을 사용하며 Codex의 lifecycle enforcement를 제공한다고 표시하지 않습니다.

## 구현 출처와 검증

Rust core·시나리오 테스트·스킬은 [0pg-mcp의 Memento 구현](https://github.com/0pg/0pg-mcp/tree/main/crates/memento)에서 가져왔습니다. 이 패키지는 원본 저장소 없이 빌드·실행할 수 있습니다. 스킬 경로와 설치 안내는 플러그인 구조에 맞췄으며 원본 Codex 설치기 전용 테스트는 플러그인 실행기 설치 테스트로 대체했습니다.

```sh
cargo fmt --manifest-path plugins/memento/core/Cargo.toml --check
cargo clippy --manifest-path plugins/memento/core/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path plugins/memento/core/Cargo.toml --all-features
python3 -m unittest discover -s plugins/memento/tests
```

격리된 `CODEX_HOME`에서 실제 Codex plugin 설치와 app-server `plugin/read`를 확인했고 `0.5.0` package의 활성화 상태, Memento와 setup 두 skill, `onboardingSkill` 경로가 노출되었습니다. 이는 선택적 설정 대화가 모든 클라이언트에서 자동 실행된다는 뜻은 아닙니다.

Runtime lifecycle 테스트는 동시 첫 사용·중단 재시도·cache 교체·모델 선택 보존·owned artifact 정리·stable Git bridge를 검증합니다. 모델 실패 테스트는 제어된 worker/tool 응답을 사용하며 실제 구버전 DB 검증과 구분합니다. 구버전 Store 검증은 `bab2e08`(`0.3.0`)과 `c007979`(`0.4.0`)의 실제 실행기로 합성 raw 대화·정정·실패·근거·Git checkpoint를 기록해 전후 행을 비교했습니다. [검증 자료와 범위](https://github.com/0pg/0pg-mcp/tree/main/docs/agent-work-context/evaluations/runtime-upgrade)를 참조하세요. Process 종료 검증은 lock 대기 지점에서 수행했으며 SQLite 개별 write 사이에 crash를 주입한 검증은 아닙니다. Codex의 선택적 onboarding UI를 모든 클라이언트에서 자동 완료시킨다는 보장도 없습니다.

배포 메타데이터: Claude Code는 `.claude-plugin/plugin.json`, Codex는 root `plugin.json`의 `extensions.com.openai`와 저장소의 `.agents/plugins/marketplace.json`을 사용합니다. `.codex-plugin/plugin.json`은 동일한 호환 메타데이터를 유지합니다. Codex 훅은 `codex-hooks/hooks.json`으로 선언해 Claude의 기본 `hooks/hooks.json` 자동 로드와 분리했습니다. [Claude Code 배포 문서](https://code.claude.com/docs/en/plugin-marketplaces), [Claude hook 로드 규칙](https://code.claude.com/docs/en/plugins-reference#hooks), [OpenAI 플러그인 패키징 문서](https://developers.openai.com/plugins/build/plugins)를 참조하세요.

## 0.6.0 원자 명제 수집

이 패키지는 머지된 [0pg-mcp `b0e9150d`](https://github.com/0pg/0pg-mcp/commit/b0e9150d6274904edc7a8ecdafe727e6d0347e77)의 Memento 코어·스킬·Codex 훅을 반영합니다.

원문 `evidence`와 독립적으로 정정·승인·검증 가능한 `claim`을 구분하며 `context_id`, 정확한 `origin`/`support` revision과 UTF-8 범위를 검사합니다. 체크포인트는 이벤트에 연결된 의미 claim 저장 후 해결합니다. Crepe는 claim에서 근거로 향하는 의존성을 보존하며 공유 원문을 통해 무관한 형제 claim 전체를 보호하지 않습니다. 프로젝트 범위와 저장 한도는 유지합니다. 수집 시 [원자 명제 가이드](skills/memento/references/atomic-claims.md)를 읽으세요.

Datalog와 구조 검사는 자연어 의미·원자성·누락 여부를 보증하지 않습니다. 마지막 모델 평가에서 S04-r1/r3의 독립 요청 결합 실패가 확인됐고, 이후 가이드 수정의 모델 추출 효과는 재평가하지 않았습니다. [상태 요약](https://github.com/0pg/0pg-mcp/blob/b0e9150d6274904edc7a8ecdafe727e6d0347e77/docs/agent-work-context/evaluations/atomic-capture-v1/pr-verification-summary.json)을 참조하세요. 위의 실제 Codex 설치 노출 확인은 0.5.0에서 수행한 과거 검증입니다.

0.6.0 패키지 검증(2026-10-06): standalone core의 fmt·clippy 및 Rust259개, Python79개, 두 skill의 공식 validator가 통과했습니다. 실제 format 1 실행기로 만든 DB의 명시적/첫 쓰기 format 2 migration에서 public25행·capture session2개·ID/revision/sequence·중복 replay·비선택 DB 보존과 구버전 writer 거절을 확인했습니다. 이 검증은 모델 추출 평가나 새 0.6.0의 Codex host 설치·활성화 검증을 의미하지 않습니다.
