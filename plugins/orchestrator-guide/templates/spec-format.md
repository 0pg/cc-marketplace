# spec.md 작성 가이드

## spec.md 작성 방법 (plan mode)

spec 정의는 **plan mode**를 통해 수행합니다:

1. **EnterPlanMode**로 plan mode 진입
2. Explore agent로 코드베이스 탐색
3. AskUserQuestion으로 요구사항 명확화
4. 본 형식에 따라 spec 작성
5. **ExitPlanMode**로 승인 요청
6. 승인 후 spec/spec.md로 저장

### spec/ 디렉터리 위치

spec/은 **모듈 또는 컴포넌트의 root**에 위치합니다:
- 모듈: 독립적으로 배포 가능한 단위
- 단일 프로젝트: 프로젝트 root
- 모노레포: 각 패키지/앱 root

```
# 단일 프로젝트
project/
├── spec/
│   ├── spec.md
│   └── task.md
└── src/

# 모노레포
monorepo/
├── packages/
│   ├── auth/
│   │   ├── spec/     ← auth 모듈 spec
│   │   └── src/
│   └── api/
│       ├── spec/     ← api 모듈 spec
│       └── src/
└── apps/
    └── web/
        ├── spec/     ← web 앱 spec
        └── src/
```

### plan mode 활용의 장점

- **탐색 단계**: 코드베이스 파악
- **명확화 단계**: 요구사항 구체화 및 결정사항 도출
- **승인 워크플로우**: ExitPlanMode로 사용자 확인

---

## 스펙 명확화 프로세스

구현 전 사용자와 함께 요구사항을 명확화하는 과정을 거쳐야 합니다.

### 활용 가능한 도구

| 도구 | 용도 |
|------|------|
| AskUserQuestion | 요구사항 명확화 질문 |
| Read | 기존 코드/문서 분석 |
| Task (Explore) | 코드베이스 탐색으로 컨텍스트 파악 |
| WebFetch/WebSearch | 외부 API/라이브러리 문서 조사 |
| mcp__context7 | 라이브러리 최신 문서 조회 |

### 명확화 과정 프로토콜 (plan mode 내에서)

1. **초기 요구사항 수집**
   - 사용자 요청의 핵심 목표 파악
   - 불명확한 부분 식별

2. **컨텍스트 조사**
   - 관련 코드/문서 탐색
   - 기존 패턴 파악
   - 외부 의존성 확인

3. **명확화 질문**
   - 식별된 불명확한 부분에 대해 AskUserQuestion 활용
   - 선택지가 있는 경우 옵션 제시
   - 기술적 제약사항 공유

4. **스펙 문서화**
   - 합의된 내용을 spec.md로 작성
   - ExitPlanMode로 사용자 승인 요청

---

## spec.md 구조

```markdown
# Spec - {feature_name}

## 개요
{1-2문장 요약}

## 배경
- 왜 이 기능이 필요한가?
- 어떤 문제를 해결하는가?

## 요구사항

### 기능 요구사항
- REQ-001: {요구사항}
- REQ-002: {요구사항}

### 비기능 요구사항
- NFR-001: {성능/보안/호환성 등}

## 제약사항
- {기술적 제약}
- {비즈니스 제약}

## 범위
### 포함
- {포함 범위}

### 제외
- {명시적 제외 범위}

## 결정사항
| 결정 | 근거 | 대안 |
|------|------|------|
| {결정 내용} | {선택 이유} | {고려했던 대안} |

## 미해결 사항
- [ ] {추후 결정 필요한 사항}
```

---

## 예시: 명확화 과정

### 사용자 요청
> "로그인 기능 추가해줘"

### 1단계: 컨텍스트 조사
```
Read: 기존 인증 관련 코드 확인
Task(Explore): 사용 중인 프레임워크/패턴 파악
WebSearch/mcp__context7: 라이브러리 문서 확인
```

### 2단계: 명확화 질문 (AskUserQuestion)
```
Q1: 인증 방식 - OAuth, JWT, Session 중 선호?
Q2: 소셜 로그인 필요 여부?
Q3: 2FA 지원 범위?
```

### 3단계: spec.md 작성
위 구조에 따라 합의된 내용 문서화

---

## spec.md → task.md 연결

spec.md의 각 요구사항(REQ-XXX)은 task.md의 TASK 항목으로 매핑되어야 합니다:

```
REQ-001 → TASK-001.x (구현)
        → VERIFY-001.x (검증)
```
