# Project Context Store

프로젝트의 컨텍스트를 자동으로 문서화하여 도메인 이해와 확률적 재현가능성을 보장하는 Claude Code 플러그인입니다.

## 핵심 개념

**확률적 재현가능성**: 코드를 삭제해도 CLAUDE.md만으로 도메인을 파악하여 여러 번 시도 시 동일한 코드를 작성할 수 있는 것

- 기존: 매번 똑같은 코드가 나와야 함 (결정론적)
- 변경: 여러 번 시도하면 동일한 코드가 나올 수 있을 정도의 정보 (확률적)

## 설치

```bash
# Claude Code에서 플러그인 설치
/install-plugin path/to/project-context-store
```

## 사용법

### 컨텍스트 생성

```
/context-generate
```

프로젝트의 소스 코드 디렉토리를 분석하여 각 디렉토리에 CLAUDE.md 파일을 생성합니다.

**동작 방식:**
1. 소스 코드 디렉토리 자동 탐지
2. 디렉토리별 도메인 분석
3. 불명확한 부분은 질문으로 확인
4. CLAUDE.md 생성

### 컨텍스트 업데이트

```
/context-update
```

코드 변경 사항을 감지하고 해당 CLAUDE.md를 업데이트합니다.

### 컨텍스트 검증

```
/context-validate
```

CLAUDE.md와 실제 코드의 일치 여부 및 도메인 이해도를 검증합니다.

## CLAUDE.md에 포함되는 정보

| 항목 | 설명 |
|------|------|
| Domain Overview | 비즈니스 영역 설명 |
| Key Concepts | 도메인 용어 정의 (Ubiquitous Language) |
| Business Rules | 비즈니스 규칙과 근거 (법규, 정책 등) |
| Domain Constants | 도메인 상수의 의미와 유래 |
| Design Rationale | 기술 선택의 이유와 대안 검토 |
| External Integrations | API 스펙, 프로토콜 정보 |
| Implementation Notes | Tribal Knowledge, 주의사항 |

## 자동 업데이트 알림

코드 변경 시 자동으로 컨텍스트 업데이트 필요 여부를 알려줍니다.
`/context-update`로 명시적으로 업데이트를 트리거할 수 있습니다.

## 성공 기준

**이 플러그인으로 생성된 CLAUDE.md로 도메인을 파악하여 여러 번 시도 시 동일한 코드를 재작성할 수 있어야 합니다.**

코드에서 직접 읽을 수 없는 "왜?"에 대한 답변이 모두 문서화되어야 합니다.

## 라이선스

MIT
