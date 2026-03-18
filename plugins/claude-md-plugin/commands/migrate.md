---
name: migrate
description: |
  claude-md-plugin 버전 업그레이드 시 기존 문서를 새 스키마에 맞게 마이그레이션합니다.
  레거시 IMPLEMENTS.md → DEVELOPERS.md 전환, CLAUDE.md 스키마 누락 섹션 추가,
  validate-schema 검증 후 /validate → /compile 연계.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Skill, AskUserQuestion]
---

# /migrate

기존 프로젝트의 문서를 현재 플러그인 버전에 맞게 마이그레이션합니다.

두 가지 마이그레이션을 자동 감지하여 처리합니다:
1. **레거시 전환**: IMPLEMENTS.md → DEVELOPERS.md (v2.x → v3.0+)
2. **스키마 업그레이드**: CLAUDE.md 누락 필수 섹션 추가 (v3.x → v4.0+)

## Triggers

- `/migrate`
- `마이그레이션`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | `.` | 프로젝트 루트 경로 |

## Workflow

### 1. 사전 확인

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

CLI 빌드 실패 시 에러 출력 후 종료.

프로젝트 내 문서 파일 수집:
```
Glob("**/CLAUDE.md", path={project_root_path})
Glob("**/IMPLEMENTS.md", path={project_root_path})
Glob("**/DEVELOPERS.md", path={project_root_path})
```

CLAUDE.md가 없으면:
> "CLAUDE.md 파일이 없습니다. 마이그레이션할 대상이 없습니다."

### 2. 마이그레이션 유형 자동 감지

수집된 파일로 마이그레이션 유형을 판별합니다:

| 감지 조건 | 마이그레이션 유형 | 설명 |
|-----------|-----------------|------|
| IMPLEMENTS.md 존재 + DEVELOPERS.md 없음 | **LEGACY_CONVERT** | v2.x → v3.0+ 전환 필요 |
| IMPLEMENTS.md + DEVELOPERS.md 공존 | **LEGACY_CLEANUP** | 전환 중 잔여 파일 정리 |
| CLAUDE.md 스키마 검증 FAIL | **SCHEMA_UPGRADE** | v3.x → v4.0+ 섹션 추가 |
| 위 해당 없음 | **UP_TO_DATE** | 마이그레이션 불필요 |

복수 유형이 동시에 감지될 수 있습니다 (예: LEGACY_CONVERT + SCHEMA_UPGRADE).

### 3. 마이그레이션 계획 표시

감지된 모든 마이그레이션 항목을 종합하여 표시합니다:

```
마이그레이션 계획
================

감지된 마이그레이션:

[1] 레거시 전환 (IMPLEMENTS.md → DEVELOPERS.md): {N}개 파일
| 파일 | 작업 |
|------|------|
| src/auth/IMPLEMENTS.md | → src/auth/DEVELOPERS.md 전환 |
| src/utils/IMPLEMENTS.md | → src/utils/DEVELOPERS.md 전환 |

[2] 스키마 업그레이드 (누락 섹션 추가): {M}개 파일
| 파일 | 누락 섹션 |
|------|----------|
| src/auth/CLAUDE.md | Async Contract, Error Taxonomy, Concurrency Model |

변경 요약:
- IMPLEMENTS.md → DEVELOPERS.md 전환: {N}개
- CLAUDE.md 섹션 추가: {M}개
- 기존 내용 변경: 없음
```

AskUserQuestion으로 확인:
```
AskUserQuestion: "위 마이그레이션을 진행하시겠습니까?"
옵션: [전체 진행, 레거시 전환만, 스키마 업그레이드만, 취소]
```

### 4. 레거시 전환 (IMPLEMENTS.md → DEVELOPERS.md)

LEGACY_CONVERT 또는 LEGACY_CLEANUP 유형이 감지된 경우 실행합니다.

#### 4.1. IMPLEMENTS.md 파싱

각 IMPLEMENTS.md를 Read하여 섹션을 추출합니다:

```
Planning Section:
  - Dependencies Direction → compile-context로 이동 (세션 한정)
  - Implementation Approach → compile-context로 이동
  - Technology Choices → compile-context로 이동

Implementation Section:
  - Algorithm → DEVELOPERS.md Decision Log
  - Key Constants → DEVELOPERS.md Decision Log
  - Error Handling → DEVELOPERS.md Decision Log + File Map
  - State Management → DEVELOPERS.md Data Structures
  - Implementation Guide → DEVELOPERS.md Operations
```

#### 4.2. DEVELOPERS.md 생성

추출된 내용을 DEVELOPERS.md 스키마에 맞게 변환합니다:

```markdown
# {directory-name}

## File Map

| 파일 | 역할 | 의존 |
|------|------|------|
(IMPLEMENTS.md의 Implementation Guide + 디렉토리 실제 파일에서 구성)

## Data Structures

(IMPLEMENTS.md의 State Management 내용 전환)
(해당 없으면 "None")

## Decision Log

(IMPLEMENTS.md의 Algorithm, Key Constants, Error Handling을 ADR 형식으로 전환)

### {결정 제목}
- **맥락**: (원본 섹션의 배경)
- **결정**: (원본 내용 요약)
- **근거**: (원본에 근거가 있으면 그대로, 없으면 코드 기반 추론)

(해당 없으면 "None")

## Operations

(IMPLEMENTS.md의 Implementation Guide 중 운영 관련 내용)
(해당 없으면 "None")
```

**변환 규칙:**

| IMPLEMENTS.md 섹션 | DEVELOPERS.md 섹션 | 변환 방법 |
|-------------------|-------------------|----------|
| Algorithm | Decision Log | 각 알고리즘을 ADR 엔트리로 변환 (맥락/결정/근거) |
| Key Constants | Decision Log | 각 상수를 ADR 엔트리로 변환 (값 + 근거) |
| Error Handling | Decision Log + File Map | 에러 전략 → Decision Log, 파일별 에러 처리 → File Map |
| State Management | Data Structures | 상태 관리 패턴을 자료구조 관계로 재구성 |
| Implementation Guide | Operations | 운영/배포/트러블슈팅 정보를 Operations로 이동 |
| Dependencies Direction | (폐기 → compile-context) | 세션 임시 파일로 이동 (영구 문서에서 제거) |
| Implementation Approach | (폐기 → compile-context) | 세션 임시 파일로 이동 |
| Technology Choices | (폐기 → compile-context) | 세션 임시 파일로 이동 |

**Planning Section 처리:**

Planning Section의 내용은 영구 문서에 보존하지 않습니다.
- 이 내용은 원래 `/impl` → `/compile` 핸드오프용 세션 임시 정보였습니다.
- 필요시 사용자가 다음 `/impl` 실행 시 compile-context로 재생성합니다.
- AskUserQuestion으로 Planning Section 폐기를 확인합니다:

```
AskUserQuestion: "IMPLEMENTS.md의 Planning Section (Dependencies Direction, Implementation Approach, Technology Choices)은
세션 임시 정보이므로 DEVELOPERS.md로 전환하지 않습니다.
이 내용을 별도 파일로 백업하시겠습니까?"
옵션: [백업 후 진행, 그냥 진행 (내용 폐기)]
```

"백업" 선택 시: `${TMP_DIR}implements-planning-backup-{dir-safe}.md`로 저장.

#### 4.3. IMPLEMENTS.md 정리

DEVELOPERS.md 생성 완료 후:

```
AskUserQuestion: "DEVELOPERS.md 생성이 완료되었습니다. 원본 IMPLEMENTS.md를 삭제하시겠습니까?"
옵션: [삭제, 유지 (수동 정리)]
```

"삭제" 선택 시:
```bash
git rm {path}/IMPLEMENTS.md
```

#### 4.4. LEGACY_CLEANUP 처리

IMPLEMENTS.md와 DEVELOPERS.md가 공존하는 경우:
1. DEVELOPERS.md를 Read하여 내용 확인
2. IMPLEMENTS.md를 Read하여 DEVELOPERS.md에 없는 정보가 있는지 확인
3. 추가 정보가 있으면 DEVELOPERS.md에 병합 제안
4. 추가 정보가 없으면 IMPLEMENTS.md 삭제만 제안

### 5. 스키마 업그레이드 (CLAUDE.md 섹션 추가)

SCHEMA_UPGRADE 유형이 감지된 경우 실행합니다.

각 FAIL 파일에 대해 `fix-schema`를 실행합니다:

```bash
for claude_md in ${failed_targets}; do
  $CLI_PATH fix-schema --file "$claude_md"
done
```

### 6. 전체 재검증

마이그레이션 후 전체 재검증:

```bash
fail_count=0
for claude_md in ${targets}; do
  result=$($CLI_PATH validate-schema --file "$claude_md" --strict 2>&1)
  if echo "$result" | grep -q '"valid":false'; then
    fail_count=$((fail_count + 1))
    echo "FAIL: $claude_md"
  fi
done
```

**전체 PASS:**
> "마이그레이션 완료. 스키마 검증: {total}/{total} PASS"

**일부 FAIL:**
> "⚠ {fail_count}개 파일이 여전히 검증 실패합니다. 수동 확인이 필요합니다."

### 7. 변경사항 Diff 표시

```bash
git diff -- "**/CLAUDE.md" "**/DEVELOPERS.md"
git status -- "**/IMPLEMENTS.md"
```

### 8. 계약 검증 (선택)

```
AskUserQuestion: "계약-코드 일치 검증(/validate)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:validate")
```

### 9. 코드 재생성 (선택)

/validate에서 위반이 발견된 경우에만 질문:

```
AskUserQuestion: "위반이 발견되었습니다. 전체 재컴파일(/compile --all)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:compile", args: "--all --conflict overwrite")
```

### 10. 결과 보고

```
마이그레이션 결과
================

[레거시 전환]
  IMPLEMENTS.md → DEVELOPERS.md: {converted}개 전환
  IMPLEMENTS.md 삭제: {deleted}개
  Planning Section 백업: {backed_up}개

[스키마 업그레이드]
  CLAUDE.md 섹션 추가: {fixed}개

[검증]
  스키마 검증: {pass}/{total} PASS
  계약 검증: {실행됨/건너뜀}
  코드 재생성: {실행됨/건너뜀}

마이그레이션 완료.
```

## DO / DON'T

**DO:**
- 마이그레이션 유형을 자동 감지 (IMPLEMENTS.md 존재 + 스키마 FAIL 동시 처리)
- 마이그레이션 전 전체 계획 표시 + 사용자 승인
- IMPLEMENTS.md → DEVELOPERS.md 변환 시 ADR 형식 준수
- Planning Section 폐기 전 백업 옵션 제공
- fix-schema CLI로 결정론적 섹션 추가
- 각 단계 결과를 명확히 표시
- /validate, /compile은 선택적 실행

**DON'T:**
- 사용자 승인 없이 파일 수정/삭제
- IMPLEMENTS.md의 Planning Section을 DEVELOPERS.md에 포함 (세션 임시 정보)
- 기존 CLAUDE.md/DEVELOPERS.md 섹션 내용 변경 (추가/전환만)
- IMPLEMENTS.md 삭제를 강제 (사용자 선택)

## Examples

<example>
<context>
v2.x에서 v4.1로 업그레이드하는 프로젝트 (IMPLEMENTS.md 존재 + 스키마 누락)
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  IMPLEMENTS.md: 2개
  DEVELOPERS.md: 1개

마이그레이션 유형 감지:
  [1] 레거시 전환 (IMPLEMENTS.md → DEVELOPERS.md): 2개
      - src/auth/IMPLEMENTS.md (DEVELOPERS.md 없음 → 전환)
      - src/utils/IMPLEMENTS.md (DEVELOPERS.md 존재 → 잔여 정리)
  [2] 스키마 업그레이드: 3개
      - 모두 Async Contract, Error Taxonomy, Concurrency Model 누락

진행하시겠습니까? [전체 진행/레거시 전환만/스키마 업그레이드만/취소]
→ 전체 진행

=== 레거시 전환 ===

[1/2] src/auth/IMPLEMENTS.md → DEVELOPERS.md
  Planning Section 백업하시겠습니까? [백업/폐기]
  → 백업
  ✓ Planning Section 백업: .claude/tmp/implements-planning-backup-src-auth.md
  ✓ DEVELOPERS.md 생성: File Map, Decision Log (3 entries), Data Structures, Operations
  원본 IMPLEMENTS.md 삭제하시겠습니까? [삭제/유지]
  → 삭제
  ✓ src/auth/IMPLEMENTS.md 삭제

[2/2] src/utils/IMPLEMENTS.md (DEVELOPERS.md 공존)
  ✓ DEVELOPERS.md에 없는 추가 정보 없음
  IMPLEMENTS.md 삭제하시겠습니까? [삭제/유지]
  → 삭제
  ✓ src/utils/IMPLEMENTS.md 삭제

=== 스키마 업그레이드 ===
  ✓ src/auth/CLAUDE.md — 3개 섹션 추가
  ✓ src/api/CLAUDE.md — 3개 섹션 추가
  ✓ src/utils/CLAUDE.md — 3개 섹션 추가

스키마 재검증: 3/3 PASS

계약 검증을 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기

마이그레이션 결과
================
레거시 전환: 1개 전환, 2개 삭제, 1개 백업
스키마 업그레이드: 3개 파일
스키마 검증: 3/3 PASS

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
v3.1에서 v4.1로 업그레이드하는 프로젝트 (스키마 누락만)
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 5개
  IMPLEMENTS.md: 0개
  DEVELOPERS.md: 5개

마이그레이션 유형 감지:
  [1] 레거시 전환: 해당 없음 (IMPLEMENTS.md 없음)
  [2] 스키마 업그레이드: 5개

진행하시겠습니까? [진행/취소]
→ 진행

스키마 업그레이드:
  ✓ 5개 파일 — 각 3개 섹션 추가

스키마 재검증: 5/5 PASS

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
이미 최신 버전인 프로젝트
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  IMPLEMENTS.md: 0개
  DEVELOPERS.md: 3개

마이그레이션 유형 감지:
  모든 문서가 현재 스키마와 호환됩니다.

마이그레이션이 필요 없습니다.
</assistant_response>
</example>
