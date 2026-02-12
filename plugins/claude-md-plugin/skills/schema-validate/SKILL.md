---
name: schema-validate
version: 1.0.0
description: |
  (internal) This skill should be used when validating CLAUDE.md files against schema rules.
  CLAUDE.md 파일이 스키마 규칙을 준수하는지 검증
user_invocable: false
allowed-tools: [Bash, Read]
---

# Schema Validate Skill

## 목적

CLAUDE.md 파일이 스키마 규칙을 준수하는지 검증.
Rust CLI `claude-md-core validate-schema`를 래핑.

## 입력

```
file_path: 검증할 CLAUDE.md 파일 경로
output_name: 출력 파일명 (디렉토리명 기반)
```

## 출력

`.claude/extract-results/{output_name}-validation.json` 파일 생성

```json
{
  "file": ".claude/extract-results/src-auth-draft.md",
  "valid": true,
  "errors": [],
  "warnings": []
}
```

또는 검증 실패 시:

```json
{
  "file": ".claude/extract-results/src-auth-draft.md",
  "valid": false,
  "errors": [
    {
      "type": "missing_section",
      "section": "Behavior",
      "message": "필수 섹션 'Behavior'가 없습니다"
    },
    {
      "type": "invalid_format",
      "section": "Exports",
      "line": 15,
      "message": "함수 시그니처 형식이 잘못되었습니다: 'validateToken'"
    }
  ],
  "warnings": [
    {
      "type": "empty_section",
      "section": "Constraints",
      "message": "Constraints 섹션이 비어있습니다"
    }
  ]
}
```

## 워크플로우

### 1. CLI 빌드 확인 및 실행

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

$CLI_PATH validate-schema \
  --file {file_path} \
  --output .claude/extract-results/{output_name}-validation.json

# --strict 플래그: INV-3 경고를 에러로 승격
# /validate에서 호출 시 사용
$CLI_PATH validate-schema \
  --file {file_path} \
  --strict \
  --output .claude/extract-results/{output_name}-validation.json
```

### 2. 결과 확인

```
Read로 .claude/extract-results/{output_name}-validation.json 읽기
JSON 내 valid 필드가 true이면 통과
false이면 errors 배열에서 이슈 목록 추출
```

## 결과 반환

**검증 통과 시:**
```
---schema-validate-result---
output_file: .claude/extract-results/{output_name}-validation.json
status: passed
errors: 0
warnings: {경고 수}
---end-schema-validate-result---
```

**검증 실패 시:**
```
---schema-validate-result---
output_file: .claude/extract-results/{output_name}-validation.json
status: failed
errors: {이슈 수}
error_details:
  - [missing_section] Behavior: 필수 섹션이 없습니다
  - [invalid_format] Exports:15: 함수 시그니처 형식이 잘못되었습니다
warnings: {경고 수}
---end-schema-validate-result---
```

## 검증 규칙

검증 규칙은 `references/schema-rules.yaml`에서 정의됨 (Single Source of Truth).

### 현재 필수 섹션 (6개)

| 섹션 | 필수 | "None" 허용 |
|------|------|-------------|
| Purpose | ✓ | ✗ |
| Exports | ✓ | ✓ |
| Behavior | ✓ | ✓ |
| Contract | ✓ | ✓ |
| Protocol | ✓ | ✓ |
| Domain Context | ✓ | ✓ |

### 조건부/선택 섹션

| 섹션 | 조건 |
|------|------|
| Structure | 하위 디렉토리/파일 있을 때 |
| Dependencies | 외부 의존성 있을 때 |
| Constraints | 제약사항 있을 때 |

*자세한 규칙과 패턴은 `references/schema-rules.yaml` 참조*

### 참조 규칙
- 부모 참조 (`../`) 금지
- 형제 참조 금지

## 제한사항

- **IMPLEMENTS.md 검증 미지원**: 현재 CLAUDE.md만 검증. IMPLEMENTS.md의 섹션 구조(Planning Section, Implementation Section) 검증은 아직 미구현 상태임 (INV-3 부분 준수).

## DO / DON'T

**DO:**
- CLI 빌드 상태 확인 후 실행
- 구조화된 결과 블록 (`---schema-validate-result---`) 반환
- errors/warnings 구분하여 보고

**DON'T:**
- 빌드 실패 시 진행하지 않음
- CLAUDE.md 파일을 직접 수정하지 않음 (검증만)
- 검증 실패 시 자동 재시도하지 않음 (호출자가 판단)

## 참조 자료

- `examples/valid/CLAUDE.md`: 검증 통과하는 CLAUDE.md 예시
- `examples/invalid-missing-section/CLAUDE.md`: 필수 섹션 누락 예시
- `examples/invalid-parent-ref/CLAUDE.md`: 부모 참조 위반 예시

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 없음 | 에러 반환 |
| CLI 실패 | CLI 에러 메시지 전달 |
| JSON 파싱 실패 | 에러 반환 |
