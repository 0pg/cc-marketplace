---
name: schema-validate
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
```

## 출력

`.claude/extract-results/{output_name}-validation.json` 파일 생성

```json
{
  "file": ".claude/extract-results/src-auth-draft.md",
  "valid": true,
  "issues": [],
  "warnings": []
}
```

또는 검증 실패 시:

```json
{
  "file": ".claude/extract-results/src-auth-draft.md",
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
  --output .claude/extract-results/{output_name}-validation.json
```

### 2. 결과 확인

```bash
validation=$(cat .claude/extract-results/{output_name}-validation.json)
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
output_file: .claude/extract-results/{output_name}-validation.json
status: passed
issues: 0
warnings: {경고 수}
---end-schema-validate-result---
```

**검증 실패 시:**
```
---schema-validate-result---
output_file: .claude/extract-results/{output_name}-validation.json
status: failed
issues: {이슈 수}
issue_details:
  - [missing_section] Behavior: 필수 섹션이 없습니다
  - [invalid_format] Exports:15: 함수 시그니처 형식이 잘못되었습니다
warnings: {경고 수}
---end-schema-validate-result---
```

## 검증 규칙

### 필수 섹션
| 섹션 | 필수 여부 | 조건 |
|------|---------|------|
| Purpose | 필수 | 항상 |
| Exports | 필수 | 항상 (없으면 "None" 명시) |
| Behavior | 필수 | 항상 |
| Structure | 조건부 | 하위 디렉토리/파일 있을 때 |
| Dependencies | 선택 | 외부 의존성 있을 때 |
| Constraints | 선택 | 제약사항 있을 때 |

### Exports 형식
```regex
^[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*[:→\->]?\s*.+$
```
- 유효: `validateToken(token: string): Promise<Claims>`
- 무효: `validateToken` (파라미터 없음)

### Behavior 형식
```regex
.+\s*[→\->]\s*.+
```
- 유효: `유효한 토큰 → Claims 객체`
- 무효: `토큰을 검증합니다`

### 참조 규칙
- 부모 참조 (`../`) 금지
- 형제 참조 금지

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 없음 | 에러 반환 |
| CLI 실패 | CLI 에러 메시지 전달 |
| JSON 파싱 실패 | 에러 반환 |
