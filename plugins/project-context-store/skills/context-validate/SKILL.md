---
name: context-validate
description: |
  CLAUDE.md와 실제 코드의 일치 여부를 검증합니다.
  오래된 정보, 누락된 컨텍스트, 불일치 항목을 찾아 보고합니다.
trigger:
  - /context-validate
  - 컨텍스트 검증
  - CLAUDE.md 검증
  - 문서 검증
tools:
  - Read
  - Glob
  - Grep
  - Task
---

# Context Validate Skill

## 목적

CLAUDE.md와 실제 코드 간의 drift(불일치)를 탐지하고 보고합니다.

## 워크플로우

### 1. CLAUDE.md 수집

```
1. Glob으로 모든 CLAUDE.md 파일 탐지
2. 각 파일의 위치와 담당 디렉토리 매핑
```

### 2. 코드와 비교

각 CLAUDE.md에 대해:

```
1. 문서화된 상수가 코드에 존재하는지 확인
2. 문서화된 값이 코드와 일치하는지 확인
3. 코드에 새로운 컨텍스트 필요 항목이 있는지 확인
4. 참조된 파일이 존재하는지 확인
```

### 3. Drift 탐지

불일치 유형:

| 유형 | 설명 | 심각도 |
|------|------|--------|
| STALE | 문서화된 항목이 코드에 없음 | High |
| MISMATCH | 값이 다름 | High |
| MISSING | 코드에 있지만 문서화되지 않음 | Medium |
| ORPHAN | 참조 파일 없음 | Low |

### 4. 보고서 생성

```markdown
=== Context Validation Report ===

## Summary
- 검증된 CLAUDE.md: 5개
- 정상: 3개
- 문제 발견: 2개

## Issues

### src/auth/CLAUDE.md

[HIGH] MISMATCH: TOKEN_EXPIRY
  - 문서: 3600
  - 코드: 7200
  - 위치: src/auth/token.rs:15

[MEDIUM] MISSING: New constant
  - MAX_REFRESH_COUNT = 5
  - 위치: src/auth/token.rs:18
  - 컨텍스트 필요

### src/api/CLAUDE.md

[HIGH] STALE: Removed constant
  - API_VERSION = "v1" (코드에서 삭제됨)

[LOW] ORPHAN: Referenced file missing
  - legacy_handler.rs (삭제됨)

## Recommendations

1. /context-update 실행하여 src/auth/CLAUDE.md 업데이트
2. src/api/CLAUDE.md에서 삭제된 항목 정리
```

## 검증 기준

### 상수/Magic Number 검증

```python
# CLAUDE.md에서 추출
documented = {
    "TOKEN_EXPIRY": {"value": 3600, "file": "token.rs"}
}

# 코드에서 추출
actual = grep_constants("src/auth/")

# 비교
for name, info in documented.items():
    if name not in actual:
        report_stale(name)
    elif actual[name] != info["value"]:
        report_mismatch(name, info["value"], actual[name])
```

### 파일 참조 검증

```python
# CLAUDE.md의 Key Files 섹션에서 추출
referenced_files = ["auth.rs", "session.rs", "token.rs"]

# 실제 존재 확인
for file in referenced_files:
    if not exists(f"src/auth/{file}"):
        report_orphan(file)
```

### 누락 컨텍스트 탐지

```python
# 코드에서 컨텍스트 필요 패턴 탐지
patterns = [
    r"const\s+\w+\s*=\s*\d+",      # 상수 선언
    r"if\s+.*[<>=]+\s*\d+",        # 조건문의 매직 넘버
    r"https?://[^\s]+",             # 외부 URL
]

# 문서화 여부 확인
for match in grep_patterns("src/auth/", patterns):
    if not documented_in_claude_md(match):
        report_missing(match)
```

## 출력 형식

검증 결과는 마크다운 형식으로 출력됩니다.
심각도별로 정렬되어 가장 중요한 문제가 먼저 표시됩니다.
