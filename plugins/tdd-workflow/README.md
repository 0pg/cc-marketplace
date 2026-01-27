# TDD Workflow

> orchestrator-guide용 TDD workflow adapter 플러그인

## 개요

이 플러그인은 orchestrator-guide의 Pluggable Workflow로 TDD 워크플로우를 제공합니다.

## 의존성

- **orchestrator-guide**: Workflow Protocol 지원 버전 (1.3.0+)
- **tdd-dev**: TDD 스킬 제공 (test-design, tdd-impl, test-reviewer)

## 워크플로우

### Phase 1: 테스트 설계
```
Skill("tdd-dev:test-design")
```
- 요구사항 분석
- 테스트 케이스 설계 (테스트 레벨별)
- `.claude/tdd-spec.md` 생성

### Phase 2: TDD 구현
```
Skill("tdd-dev:tdd-impl")
```
- Red-Green-Refactor 사이클
- 각 REQ에 대해 테스트 먼저 작성 -> 구현

### 검증
- 리뷰어: `tdd-dev:test-reviewer`
- 매핑: REQ -> Test (직접)

## 트리거

다음 키워드로 이 워크플로우가 선택됩니다:
- "TDD로 구현해줘"
- "테스트 주도 개발"
- "Red-Green-Refactor로"

## 설치 및 설정

```bash
# 1. 플러그인 설치
claude plugins install tdd-workflow

# 2. 프로젝트에 워크플로우 등록
/tdd-workflow:setup
```

## 사용 예시

### 워크플로우 선택 (2개 이상 등록 시)

```
User: "로그인 기능 구현해줘"

Orchestrator:
1. project-config에서 workflows 확인: [default, tdd-workflow]
2. 2개 이상 -> AskUserQuestion:
   "어떤 워크플로우를 사용하시겠습니까?"
   - Default (spec+task)
   - TDD Workflow
3. 사용자 선택에 따라 진행
```

### 명시적 워크플로우 요청

```
User: "TDD로 로그인 기능 구현해줘"

Orchestrator:
1. "TDD로" 키워드 감지 -> tdd-workflow 직접 선택 (질문 없음)
2. Phase 1: Skill("tdd-dev:test-design") -> tdd-spec.md 생성
3. Phase 2: Skill("tdd-dev:tdd-impl") -> Red-Green-Refactor 실행
4. Verification: tdd-dev:test-reviewer -> REQ-Test 매핑 검증
```

## Convention 플러그인 연동

코드 작성 시 프로젝트에 설치된 Convention 플러그인을 참조합니다:

| 조합 | 결과 |
|------|------|
| tdd-workflow + rust-convention | Rust TDD 개발 |
| tdd-workflow + typescript-convention | TypeScript TDD 개발 |
| tdd-workflow (단독) | 언어 표준 스타일 적용 |

## 파일 구조

```
tdd-workflow/
├── .claude-plugin/
│   └── plugin.json               # 플러그인 매니페스트
├── commands/
│   └── setup.md                  # 워크플로우 등록 커맨드
├── skills/
│   └── tdd-orchestration/
│       ├── SKILL.md              # 워크플로우 정의
│       └── references/
│           └── verification-chain.md  # TDD 검증 체인
└── README.md                     # 이 파일
```

## Default Workflow와의 차이점

| 구분 | Default Workflow | TDD Workflow |
|------|------------------|--------------|
| 명세 파일 | spec.md + task.md | tdd-spec.md |
| 구현 순서 | 명세 -> 구현 -> 테스트 | 테스트 -> 구현 |
| 매핑 | REQ -> VERIFY -> Test | REQ -> Test (직접) |
| 리뷰어 | test-quality-reviewer | test-reviewer |
