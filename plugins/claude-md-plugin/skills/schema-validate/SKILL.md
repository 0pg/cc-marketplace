---
name: schema-validate
version: 1.0.0
description: (internal) CLAUDE.md 파일이 스키마 규칙을 준수하는지 검증
allowed-tools: [Bash, Read]
---

# Schema Validate Skill

## 목적

CLAUDE.md 파일이 스키마 규칙을 준수하는지 검증합니다.
Rust CLI `claude-md-core validate-schema`를 래핑합니다.

## 입력

```
file_path: 검증할 CLAUDE.md 파일 경로
output_name: 출력 파일명 (디렉토리명 기반)
with_index: (선택) 프로젝트 루트 경로 — 지정 시 symbol index를 빌드하여 cross-reference 실제 해석 검증
```

## 출력

`.claude/tmp/{session-id}-validation-{target}.json` 파일 생성

```json
{
  "file": ".claude/tmp/{session-id}-decompile-src-auth-claude.md",
  "valid": true,
  "errors": [],
  "warnings": [],
  "unresolved_references": []
}
```

또는 검증 실패 시:

```json
{
  "file": ".claude/tmp/{session-id}-decompile-src-auth-claude.md",
  "valid": false,
  "issues": [
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

### 1. CLI 실행

```bash
claude-md-core validate-schema \
  --file {file_path} \
  --output .claude/tmp/{session-id}-validation-{target}.json
```

### 1-b. Cross-Reference 검증 (v2, --with-index)

`with_index`가 지정된 경우, symbol index를 빌드하여 cross-reference가 실제로 해석 가능한지 검증합니다.

```bash
claude-md-core validate-schema \
  --file {file_path} \
  --output .claude/tmp/{session-id}-validation-{target}.json \
  --with-index {project_root}
```

**--with-index 유무에 따른 동작 차이:**

| 항목 | without (기본) | with --with-index |
|------|---------------|-------------------|
| 필수 섹션 확인 | O | O |
| 섹션 형식 검증 | O | O |
| cross-ref 구문 검증 | O (warning) | O |
| cross-ref 실제 해석 | X | O — symbol index 대조, 미해석 시 error |

미해석 cross-reference가 발견되면 `valid: false`가 되며, `unresolved_references` 배열에 상세 정보가 포함됩니다.

### 2. 결과 확인

```bash
validation=$(cat .claude/tmp/{session-id}-validation-{target}.json)
if [ "$(echo $validation | jq -r '.valid')" = "true" ]; then
    echo "Validation passed"
else
    echo "Validation failed"
    echo "Issues:"
    echo $validation | jq -r '.issues[] | "  - \(.section): \(.message)"'
fi
```

## 결과 반환

**검증 통과 시:**
```
---schema-validate-result---
output_file: .claude/tmp/{session-id}-validation-{target}.json
status: passed
issues: 0
warnings: {경고 수}
unresolved_references: 0
---end-schema-validate-result---
```

**검증 실패 시:**
```
---schema-validate-result---
output_file: .claude/tmp/{session-id}-validation-{target}.json
status: failed
issues: {이슈 수}
issue_details:
  - [missing_section] Behavior: 필수 섹션이 없습니다
  - [invalid_format] Exports:15: 함수 시그니처 형식이 잘못되었습니다
warnings: {경고 수}
unresolved_references: {N}
---end-schema-validate-result---
```

## 검증 규칙

검증 규칙은 `references/schema-rules.yaml`에서 정의됩니다 (Single Source of Truth).

### 현재 필수 섹션 (7개)

| 섹션 | 필수 | "None" 허용 |
|------|------|-------------|
| Purpose | ✓ | ✗ |
| Summary | ✓ | ✗ |
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

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 없음 | 에러 반환 |
| CLI 실패 | CLI 에러 메시지 전달 |
| JSON 파싱 실패 | 에러 반환 |
