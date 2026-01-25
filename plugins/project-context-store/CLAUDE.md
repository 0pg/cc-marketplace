# Project Context Store Plugin

## 목적

**코드를 삭제해도 Context만으로 동일한 코드를 재현할 수 있도록 하는 것.**

이것이 플러그인의 유일한 존재 이유입니다.

## 핵심 개념

### Context란?

코드를 재현하는데 필요하지만 **코드 자체에서 읽어낼 수 없는 정보**:

| 구분 | 예시 | 왜 필요한가 |
|------|------|------------|
| Magic Numbers | `TIMEOUT = 30` | "왜 30인가?" - 스펙, 실험 결과, 외부 제약 등 |
| 도메인 규칙 | `if age < 19:` | "왜 19인가?" - 법규, 정책, 비즈니스 요구사항 |
| 특이 조건분기 | `if len > 1000 and type == 'A':` | "왜 이 조합인가?" - 레거시, 성능, 버그 우회 등 |
| 설계 결정 | "왜 이 라이브러리를 선택했나" | 대안 검토 결과, 트레이드오프 |
| 외부 연동 | API 엔드포인트, 프로토콜 | 스펙 문서 참조, 버전 정보 |
| Tribal Knowledge | "이렇게 하면 안 되는 이유" | 과거 실패 경험, 숨겨진 제약 |

### Context가 아닌 것

코드에서 직접 읽어낼 수 있는 정보:
- 함수/클래스 시그니처 (코드가 정의)
- 타입 정보 (코드가 정의)
- 제어 흐름 자체 (코드가 보여줌)
- 일반적인 프로그래밍 패턴 (상식)

## 아키텍처

```
Skills (사용자 진입점)          Agents (Task로 실행)
├── context-generate ────────→ context-generator
├── context-update ──────────→ context-generator
└── context-validate ────────→ (직접 처리)
```

### Skill vs Agent 관계

| 구분 | 역할 |
|------|------|
| Skill | 사용자 진입점, 디렉토리 탐지, Task 생성/조율 |
| Agent | 단일 디렉토리 분석, 사용자 질의, CLAUDE.md 작성 |

## 워크플로우

1. 사용자가 `/context-generate` 호출
2. Skill이 소스 코드 디렉토리 탐지
3. 디렉토리별 독립 Task 생성 (병렬 처리)
4. 각 Task에서 context-generator 에이전트 실행
5. 불명확한 부분은 AskUserQuestion으로 질의
6. 결과 수집 및 보고

## 성공 기준

**테스트**: 코드를 모두 삭제하고 CLAUDE.md만 주었을 때, Claude가 동일한 코드를 작성할 수 있어야 함.

## 파일 구조

```
plugins/project-context-store/
├── .claude-plugin/
│   ├── plugin.json          # 플러그인 매니페스트
│   └── marketplace.json     # 마켓플레이스 메타데이터
├── CLAUDE.md                # 이 파일 (개발 가이드)
├── README.md                # 사용자 문서
├── skills/
│   ├── context-generate/    # 컨텍스트 생성 진입점
│   ├── context-update/      # 변경 감지 및 업데이트
│   └── context-validate/    # 검증
├── agents/
│   └── context-generator.md # 핵심 분석/문서화 에이전트
├── hooks/
│   ├── hooks.json           # 변경 감지 훅
│   └── detect-code-changes.md
└── templates/
    └── claude-md-template.md # CLAUDE.md 구조 템플릿
```

## 개발 원칙

1. **추측 금지**: 확신 없으면 반드시 사용자에게 질문
2. **격리**: 각 에이전트는 자신의 담당 디렉토리만 처리
3. **재현성**: 생성된 문서만으로 코드 재현 가능해야 함
