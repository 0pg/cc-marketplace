# cc-marketplace

Claude Code 플러그인 마켓플레이스입니다. Memento는 Codex에서도 배포·설치할 수 있습니다.

## 플러그인 목록

| 플러그인 | 버전 | 카테고리 | 설명 |
|---------|------|---------|------|
| [claude-md-plugin](./plugins/claude-md-plugin) | 18.1.0 | documentation | CLAUDE.md Primary SSOT document-code sync plugin. /spec, /dev, /validate, /decompile, /bugfix, /impact, /inspect |
| [project-init](./plugins/project-init) | 1.0.0 | development | Multi-language 프로젝트 초기 설정 플러그인 |
| [memento](./plugins/memento) | 0.6.0 | development | Claude Code·Codex 작업 맥락 기록 및 근거 조회, Codex 체크포인트 훅 |

## 슬래시 커맨드

### claude-md-plugin
- `/spec` — 요구사항 → CLAUDE.md + DEVELOPERS.md 정의 (`--resync` 로 Constraints만 재생성)
- `/dev` — CLAUDE.md 기반 TDD 코드 생성
- `/validate` — 문서-코드 일관성 검증
- `/decompile` — 소스코드 → CLAUDE.md 추출
- `/bugfix` — 3-layer 근본 원인 분석 및 수정
- `/impact` — 변경 영향 분석 (모듈 의존성 그래프)
- `/inspect` — 통합 read-only 점검 (`--focus health | quality | feasibility | all`)
- `/autodev` — 자율 end-to-end 실행 (spec + dev 파이프라인)
- `/migrate` — 버전 업그레이드 마이그레이션

### project-init
- `/project-init` — 멀티 언어 프로젝트 초기화

## 설치

Claude Code:

```sh
claude plugin marketplace add 0pg/cc-marketplace
claude plugin install memento@jhk-plugins
```

Codex:

```sh
codex plugin marketplace add 0pg/cc-marketplace
codex plugin add memento@jhk-plugins
```

Memento는 첫 실제 명령에서 실행기를 준비합니다. [준비·업데이트 안내](./plugins/memento/README.md#자동-실행기-준비와-업데이트)를 참조하세요. 다른 Claude Code 플러그인은 설치 명령의 `memento`를 해당 이름으로 바꿉니다.

## 버전 관리

[SemVer](https://semver.org/) 규칙을 따릅니다.

- **PATCH**: 버그 수정, 문서 오타
- **MINOR**: 기능 추가, 기존 기능 개선
- **MAJOR**: 호환성이 깨지는 변경
