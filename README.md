# cc-marketplace

Claude Code 플러그인 마켓플레이스입니다.

## 플러그인 목록

| 플러그인 | 버전 | 카테고리 | 설명 |
|---------|------|---------|------|
| [orchestrator-guide](./plugins/orchestrator-guide) | 1.6.2 | development | Multi-agent orchestration framework. 복잡한 태스크를 분해하고 에이전트에 위임 |
| [claude-md-plugin](./plugins/claude-md-plugin) | 11.1.0 | documentation | CLAUDE.md 기반 document-code 동기화 플러그인 |
| [project-context-store](./plugins/project-context-store) | 1.6.1 | documentation | 프로젝트 컨텍스트를 CLAUDE.md로 자동 문서화 |
| [tdd-dev](./plugins/tdd-dev) | 1.4.1 | development | TDD/ATDD 원칙 가이드. Outside-In TDD로 테스트가 인터페이스를 정의 |
| [tdd-workflow](./plugins/tdd-workflow) | 1.0.0 | development | orchestrator-guide용 TDD 워크플로우 어댑터 |
| [project-init](./plugins/project-init) | 1.0.0 | development | Multi-language 프로젝트 초기 설정 플러그인 |
| [project-setup](./plugins/project-setup) | 2.0.0 | development | 프로젝트 빌드/테스트 커맨드를 파악하고 CLAUDE.md에 저장 |

## 슬래시 커맨드

### orchestrator-guide
- `/orchestrator` (`/orch`) — 복잡한 태스크 분해 및 에이전트 위임 오케스트레이션
- `/planner` — 계획 및 스펙 생성
- `/delegator` — 위임 프롬프트 생성

### claude-md-plugin
- `/spec` — 요구사항 → CLAUDE.md 정의
- `/dev` — CLAUDE.md 기반 코드 생성
- `/validate` — 문서-코드 일관성 검증
- `/decompile` — 소스코드 → CLAUDE.md 추출
- `/bugfix` — 버그 분석 및 수정
- `/autodev` — 자율 end-to-end 실행 (spec + dev 파이프라인)
- `/migrate` — 버전 업그레이드 마이그레이션

### project-context-store
- `/context-generate` — 소스코드 디렉토리의 CLAUDE.md 생성
- `/context-update` — 코드 변경 감지 후 CLAUDE.md 업데이트
- `/context-validate` — 드리프트 감지 + 재현가능성 검증

### tdd-dev
- `/test-design` — 테스트 설계 및 tdd-spec.md 생성
- `/tdd-impl` — Red-Green-Refactor 사이클 기반 TDD 구현

### tdd-workflow
- `/tdd-orchestration` — TDD 오케스트레이션 (테스트 설계 → TDD 구현)

### project-init
- `/project-init` — 멀티 언어 프로젝트 초기화

### project-setup
- `/project-setup` — 빌드/테스트 커맨드 감지 및 CLAUDE.md 저장

## 설치

```bash
claude mcp add-json cc-marketplace '{"type":"stdio","command":"claude","args":["mcp","serve","/path/to/cc-marketplace/.claude-plugin/marketplace.json"]}'
```

## 버전 관리

[SemVer](https://semver.org/) 규칙을 따릅니다.

- **PATCH**: 버그 수정, 문서 오타
- **MINOR**: 기능 추가, 기존 기능 개선
- **MAJOR**: 호환성이 깨지는 변경
