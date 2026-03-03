# TDD Spec Format

`.claude/tdd-spec.md` 파일의 형식을 정의합니다.

## 목적

- 요구사항을 구조화된 형식으로 영속화
- test-reviewer 에이전트가 파싱할 수 있는 표준 형식 제공
- 요구사항 -> 테스트 추적성 확보

## 파일 위치

```
{project-root}/.claude/tdd-spec.md
```

## 형식

```markdown
# TDD Spec

> 자동 생성된 파일입니다. test-design 스킬 실행 시 생성/업데이트됩니다.

## Context

- **생성일**: YYYY-MM-DD
- **마지막 업데이트**: YYYY-MM-DD
- **대상 기능**: [기능 이름 또는 모듈]
- **Exports Source**: [CLAUDE.md 경로] (STRUCT-XXX 추출 시)

## Requirements

### REQ-001: [요구사항 제목]

- **설명**: [상세 설명]
- **입력**: [입력 조건]
- **출력**: [기대 출력]
- **엣지 케이스**:
  - [경계 조건 1]
  - [경계 조건 2]
- **에러 케이스**:
  - [에러 상황 1]
  - [에러 상황 2]

### REQ-002: [요구사항 제목]

- **설명**: ...
- **입력**: ...
- **출력**: ...
- **엣지 케이스**: ...
- **에러 케이스**: ...

## Verification Criteria

### REQ-001
- [ ] Happy path 테스트 작성
- [ ] 엣지 케이스 테스트 작성
- [ ] 에러 케이스 테스트 작성

### REQ-002
- [ ] Happy path 테스트 작성
- [ ] ...
```

### STRUCT-XXX: 구조적 불변식 (CLAUDE.md Exports 기반)

CLAUDE.md가 존재할 때 Exports에서 자동 추출됩니다.
`## Requirements` 섹션 내에서 REQ-XXX 항목 뒤에 STRUCT-XXX 항목을 배치합니다.

#### 형식

```markdown
### STRUCT-001: [Export 이름] 존재 및 시그니처 검증

- **유형**: function | type | class | enum | variable
- **Export**: [Export 이름]
- **시그니처**: [전체 시그니처]
- **검증 항목**:
  - 존재(Existence): 심볼이 public으로 접근 가능
  - 시그니처(Signature): 파라미터 타입/반환 타입 일치
  - 계약(Contract): [Contract 섹션 사전/사후조건] (있을 때)
```

#### 예시

```markdown
### STRUCT-001: validateToken 존재 및 시그니처 검증

- **유형**: function
- **Export**: validateToken
- **시그니처**: `(token: string) => Promise<Claims>`
- **검증 항목**:
  - 존재: `validateToken`이 모듈에서 export됨
  - 시그니처: string 파라미터를 받고 Promise<Claims>을 반환
  - 계약: token이 빈 문자열이면 InvalidTokenError throw
  - 계약: 반환된 Claims는 항상 exp 필드를 포함

### STRUCT-002: Claims 타입 존재 검증

- **유형**: type
- **Export**: Claims
- **시그니처**: `interface Claims { sub: string; exp: number; iat: number; }`
- **검증 항목**:
  - 존재: `Claims` 타입이 모듈에서 export됨
  - 시그니처: sub, exp, iat 필드가 올바른 타입으로 존재
```

#### STRUCT vs REQ

| 특성 | STRUCT-XXX | REQ-XXX |
|------|-----------|---------|
| 소스 | CLAUDE.md Exports | 요구사항/Behaviors |
| 변경 빈도 | API 계약 변경 시 (드묾) | 요구사항 변경 시 |
| 실패 의미 | 공개 인터페이스 깨짐 (Breaking Change) | 동작이 기대와 다름 |
| 추출 방법 | Exports에서 기계적 추출 | 수동 설계 필요 |

## 필드 설명

### Context 섹션

| 필드 | 설명 |
|------|------|
| 생성일 | spec 파일 최초 생성 날짜 |
| 마지막 업데이트 | 가장 최근 수정 날짜 |
| 대상 기능 | 이 spec이 다루는 기능/모듈 이름 |
| Exports Source | (선택) CLAUDE.md 경로 — STRUCT-XXX 추출 시 기록 |

### Requirements 섹션

| 필드 | 필수 | 설명 |
|------|------|------|
| REQ-XXX | O | 고유 식별자 (3자리 숫자) |
| 설명 | O | 요구사항의 상세 설명 |
| 입력 | O | 입력 데이터/조건 |
| 출력 | O | 기대되는 출력/결과 |
| 엣지 케이스 | - | 경계 조건 목록 |
| 에러 케이스 | - | 에러 상황 목록 |

### Verification Criteria 섹션

각 요구사항에 대한 테스트 완료 체크리스트입니다.
test-reviewer 에이전트가 이 섹션의 상태를 리포트에 포함합니다.

**STRUCT-XXX 체크리스트 (CLAUDE.md Exports가 있을 때):**
```markdown
### STRUCT-001
- [ ] 존재(Existence) 테스트 작성
- [ ] 시그니처(Signature) 테스트 작성
- [ ] 계약(Contract) 테스트 작성 (해당 시)
```

## 예시

```markdown
# TDD Spec

> 자동 생성된 파일입니다. test-design 스킬 실행 시 생성/업데이트됩니다.

## Context

- **생성일**: 2024-01-15
- **마지막 업데이트**: 2024-01-15
- **대상 기능**: 사용자 인증 모듈
- **Exports Source**: src/auth/CLAUDE.md

## Requirements

### REQ-001: 사용자 로그인

- **설명**: 이메일과 비밀번호로 사용자 인증을 수행한다
- **입력**: 이메일(string), 비밀번호(string)
- **출력**: 인증 토큰(JWT) 또는 에러
- **엣지 케이스**:
  - 빈 이메일
  - 빈 비밀번호
  - 이메일 형식 오류
- **에러 케이스**:
  - 존재하지 않는 사용자
  - 비밀번호 불일치
  - 계정 잠금 상태

### REQ-002: 토큰 갱신

- **설명**: 만료된 토큰을 갱신하여 새 토큰을 발급한다
- **입력**: 리프레시 토큰(string)
- **출력**: 새 인증 토큰(JWT)
- **엣지 케이스**:
  - 토큰 만료 직전
  - 토큰 만료 직후
- **에러 케이스**:
  - 유효하지 않은 리프레시 토큰
  - 취소된 리프레시 토큰

### STRUCT-001: authenticate 존재 및 시그니처 검증

- **유형**: function
- **Export**: authenticate
- **시그니처**: `(email: string, password: string) => Promise<AuthToken>`
- **검증 항목**:
  - 존재: `authenticate`가 모듈에서 export됨
  - 시그니처: string 2개를 받고 Promise<AuthToken>을 반환
  - 계약: email이 빈 문자열이면 ValidationError throw

## Verification Criteria

### REQ-001
- [x] Happy path 테스트 작성
- [x] 엣지 케이스 테스트 작성
- [ ] 에러 케이스 테스트 작성

### REQ-002
- [ ] Happy path 테스트 작성
- [ ] 엣지 케이스 테스트 작성
- [ ] 에러 케이스 테스트 작성

### STRUCT-001
- [ ] 존재(Existence) 테스트 작성
- [ ] 시그니처(Signature) 테스트 작성
- [ ] 계약(Contract) 테스트 작성
```

## 파싱 가이드

test-reviewer 에이전트가 이 파일을 파싱할 때:

1. `### REQ-XXX:` 패턴으로 행위적 요구사항 식별
2. `### STRUCT-XXX:` 패턴으로 구조적 불변식 식별
3. 각 요구사항의 하위 필드를 `- **필드명**:` 패턴으로 추출
4. 리스트 항목은 `-` 로 시작하는 라인으로 수집
5. Verification Criteria는 `- [ ]` 또는 `- [x]` 패턴으로 체크리스트 상태 확인
