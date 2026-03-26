<!--
  decompile-templates.md
  Consolidated reference for the decompiler agent.
  Contains: Phase 3-4 workflow, CLAUDE.md v7 templates,
  Domain Context/Requirements extraction guides,
  DEVELOPERS.md Constraints/Technical Context extraction guides,
  and CLI output JSON structures.

  v7: CLAUDE.md Requirements (PM-level, business language).
  DEVELOPERS.md Constraints (precise I/O contracts, test source) + Technical Context.

  Loaded at runtime by the decompiler agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/decompile/references/decompile-templates.md"
-->

# Decompiler Templates & Reference

## Phase 3: Domain Context 질문 (필요시)

분석 결과에서 불명확한 부분이 있으면 사용자에게 질문합니다.

**질문 안 함** (코드에서 추론 가능):
- 함수명에서 목적이 명확한 경우
- 상수 값을 계산할 수 있는 경우
- 표준 패턴을 따르는 경우

**질문 함** (코드만으로 불명확):
- 비표준 매직 넘버의 비즈니스 의미
- 도메인 전문 용어
- 컨벤션을 벗어난 구현의 이유
- **Domain Context 관련**: 결정 근거, 외부 제약, 호환성 요구
- **Constraints 관련**: 코드에 하드코딩된 제한의 근거

불명확한 부분이 있으면 AskUserQuestion으로 사용자에게 질문합니다. 예를 들어, 매직 넘버의 비즈니스 배경을 확인하기 위해 "GRACE_PERIOD_DAYS = 7의 비즈니스 배경이 있나요?" 같은 질문을 합니다. 옵션으로 "법적 요구사항", "비즈니스 정책", "기술적 제약" 등을 제시합니다.

### Domain Context 질문 (CLAUDE.md용 - 코드에서 추출 불가)

Domain Context는 코드에서 추론할 수 없는 "왜?"에 해당합니다.
상수 값, 설계 결정, 특이한 구현이 있을 때 반드시 질문합니다:

1. **상수 값의 결정 근거 (Decision Rationale)**: 매직 넘버가 발견되면 값을 선택한 이유를 질문합니다. 옵션: 컴플라이언스(PCI-DSS, GDPR 등), SLA/계약, 내부 정책, 기술적 산출
2. **외부 제약 조건 (Constraints)**: 지켜야 할 외부 제약이 있는지 질문합니다. 옵션: 있음(규제, 라이선스 등), 없음
3. **호환성 요구 (Compatibility)**: 레거시 패턴이 발견되면 호환성 요구가 있는지 질문합니다. 옵션: 있음(특정 버전/형식 지원 필요), 없음

질문이 있으면 AskUserQuestion으로 한 번에 전달합니다.

### Constraints 추출 패턴 (2-Track 생성)

코드에서 Constraints를 추출할 때 **PM-level**과 **Developer-level** 2가지 출력을 생성합니다:

**추출 소스:**

1. **Guard clauses / Validation**: `if (!x) throw`, `assert`, `require` → 제약 조건
2. **상수 제한**: `MAX_*`, `MIN_*`, `LIMIT_*` 패턴 → 수치 제약
3. **타입 제약**: `extends`, `implements`, generic bounds → 타입 제약
4. **주석 태그**: `@constraint`, `@precondition`, `@invariant` → 명시적 제약
5. **환경 종속**: 특정 환경변수, 설정값 의존 → 실행 제약

**PM-level → CLAUDE.md Requirements** (사용자 관점, 검증 가능):
- `토큰 만료 최대 7일` → "토큰은 발급 후 7일 이내에 만료되어야 한다"
- `동시 세션 최대 5개` → "사용자당 동시 활성 세션은 5개로 제한한다"

**Developer-level → DEVELOPERS.md Constraints** (정밀한 입출력 계약, 테스트 생성 원천):
- `토큰 만료 최대 7일` → "TokenService.issue(user) → token.expiresAt <= now + 7d"
- `동시 세션 최대 5개` → "SessionManager.create(userId) throws MaxSessionError when active >= 5"

없으면 각각 `None` 명시.

## Phase 4: CLAUDE.md 초안 생성 (v7 스키마)

`format-analysis` 출력(`{output_name}-summary.md`)을 primary data source로 사용하여 CLAUDE.md를 생성합니다.
소스 파일 직접 읽기는 Domain Context 파악에만 사용합니다:

1. **자식 CLAUDE.md Purpose 추출**: 각 자식 CLAUDE.md 파일이 존재하면 읽어서 Purpose 섹션을 추출합니다.
2. **CLAUDE.md 템플릿 생성**: 다음 템플릿에 맞게 CLAUDE.md를 작성합니다:

```markdown
# {directory_name}

## Purpose

{분석에서 추출한 목적 또는 사용자 응답}

## Requirements

{코드에서 추출한 비즈니스 요구사항 (PM 관점) 또는 "None"}

## Domain Context

{사용자 응답 기반 도메인 컨텍스트 또는 "None"}
```

**v7 스키마 필수 섹션:**
- **Purpose** (필수, None 불가): 모듈의 책임을 1-2문장으로 명시
- **Requirements** (필수, None 허용): 비즈니스 요구사항 (사용자 관점). 자기완결적.
- **Domain Context** (필수, None 허용): 핵심 맥락 요약. 2-3문장.

**조건부 섹션:**
- **Instructions**: project root에만 (is_project_root)
- **Conventions**: project/module root에만 (is_project_or_module_root), 6개 필수 서브섹션

3. **대상 디렉토리에 직접 Write** (${TMP_DIR} 미사용)

---

## Requirements 형식 가이드 (CLAUDE.md)

코드에서 추출한 비즈니스 요구사항을 PM 관점으로 구조화:

```markdown
## Requirements

- 토큰은 발급 후 7일 이내에 만료되어야 한다
- refresh token은 안전한 저장소에만 보관한다
- 사용자당 동시 활성 세션은 5개로 제한한다
- CSV 파일은 UTF-8 인코딩만 지원한다
```

**규칙:**
- 사용자/비즈니스 관점에서 검증 가능한 문장으로 작성
- 기술적 구현 세부사항 (함수명, 인터페이스 등) 배제
- 근거가 있으면 괄호 안에 짧게 추가 (e.g., `(PCI-DSS)`)
- 상위 모듈의 요구사항도 해당 모듈에 적용되면 반복 기재 (자기완결)
- 요구사항이 없으면 `None` 명시

---

## DEVELOPERS.md 추출 가이드 (v7)

CLAUDE.md와 함께 DEVELOPERS.md도 생성합니다. DEVELOPERS.md는 Derived Spec으로, 테스트 생성의 주요 원천입니다.

### Constraints 형식 가이드 (DEVELOPERS.md)

코드에서 추출한 정밀한 입출력 계약을 구조화:

```markdown
## Constraints

- TokenService.issue(user) → token.expiresAt <= now + 7d
- TokenStore.save(token) requires storage.isSecure == true
- SessionManager.create(userId) throws MaxSessionError when activeSessions >= 5
- CsvParser.parse(input) requires input.encoding == UTF-8
```

**규칙:**
- 함수명, 파라미터, 반환값을 포함한 정밀한 계약으로 작성
- `→` (반환/결과), `requires` (전제조건), `throws` (에러 조건) 패턴 사용
- 이 제약들이 `/compile`의 테스트 생성 원천이 됨
- 제약이 없으면 `None` 명시

### Technical Context 형식 가이드 (DEVELOPERS.md)

기술 선택과 근거를 구조화:

```markdown
## Technical Context

IdP SLA가 500ms이므로 타임아웃을 SLA × 4 = 2000ms로 설정합니다.
Redis 캐시를 사용하여 인증 지연을 최소화하며, TTL은 토큰 만료 - 10분으로 설정합니다.
```

**규칙:**
- 기술 선택 + 구체적 수치 + 선택 근거 포함
- 코드에서 추출한 상수/설정의 산출 근거에 집중
- 맥락이 없으면 `None` 명시

### Decision Log / Operations (DEVELOPERS.md, optional)

- **Decision Log**: ADR 스타일 — 맥락/결정/근거
- **Operations**: 배포 주의사항, 모니터링, gotchas
- 해당 내용이 없으면 섹션 자체를 생략

---

## Domain Context 형식 가이드

```markdown
## Domain Context

JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책을 적용합니다.
Redis 캐시를 사용하여 인증 지연을 최소화합니다.
```

**규칙:**
- 2-3문장으로 핵심 맥락만 요약
- 코드만으로는 알 수 없는 "왜(WHY)"에 집중
- 히스토리(변경 이력)는 포함하지 않음 — git에 의존
- 맥락이 없으면 `None` 명시

---
