# cc-marketplace

Claude Code 플러그인 마켓플레이스입니다.

## 플러그인 목록

| 플러그인 | 버전 | 카테고리 | 설명 |
|---------|------|---------|------|
| [claude-md-plugin](./plugins/claude-md-plugin) | 15.0.0 | documentation | CLAUDE.md Primary SSOT document-code sync plugin. /spec, /dev, /validate, /decompile + Phase 2 PM/PO skills |
| [project-init](./plugins/project-init) | 1.0.0 | development | Multi-language 프로젝트 초기 설정 플러그인 |

## 슬래시 커맨드

### claude-md-plugin
- `/spec` — 요구사항 → CLAUDE.md + DEVELOPERS.md 정의
- `/dev` — CLAUDE.md 기반 TDD 코드 생성
- `/validate` — 문서-코드 일관성 검증
- `/decompile` — 소스코드 → CLAUDE.md 추출
- `/bugfix` — 3-layer 근본 원인 분석 및 수정
- `/sync` — Requirements 변경 후 DEVELOPERS.md Constraints 부분 갱신
- `/impact` — 변경 영향 분석 (모듈 의존성 그래프)
- `/impl-review` — CLAUDE.md + DEVELOPERS.md 품질 리뷰
- `/status` — 프로젝트 건강 대시보드
- `/autodev` — 자율 end-to-end 실행 (spec + dev 파이프라인)
- `/migrate` — 버전 업그레이드 마이그레이션

### project-init
- `/project-init` — 멀티 언어 프로젝트 초기화

## 설치

```bash
claude mcp add-json cc-marketplace '{"type":"stdio","command":"claude","args":["mcp","serve","/path/to/cc-marketplace/.claude-plugin/marketplace.json"]}'
```

## 버전 관리

[SemVer](https://semver.org/) 규칙을 따릅니다.

- **PATCH**: 버그 수정, 문서 오타
- **MINOR**: 기능 추가, 기존 기능 개선
- **MAJOR**: 호환성이 깨지는 변경
