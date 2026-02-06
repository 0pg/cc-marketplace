# v1/v2 Compatibility Rules

## v2 감지 기준

파일 첫 5줄에 `<!-- schema: 2.0 -->` 마커 존재 여부로 판단합니다.

## 호환성 규칙

| 상황 | 동작 |
|------|------|
| v1 CLAUDE.md (마커 없음) | 기존 방식 그대로 동작. Cross-reference resolution 건너뜀 |
| v2 CLAUDE.md (`<!-- schema: 2.0 -->`) | Cross-reference resolution 활성, symbol-index 활용 |
| v1/v2 혼합 프로젝트 | 파일 단위로 판단. v1은 v1 방식, v2는 v2 방식 |

## 명령어별 동작

| 명령어 | 신규 생성 | 기존 v1 | 기존 v2 |
|--------|----------|---------|---------|
| `/spec` | v2 형식 | 기존 형식 유지 (v2 전환 제안 가능) | v2 유지 |
| `/compile` | 자동 판단 | v1 방식 compile | v2 방식 + symbol-index |
| `/decompile` | v2 형식 | 기존 v1 유지 (migrate CLI로 변환) | v2 유지 |
| `/validate` | - | v1 방식 검증 | v2 + cross-reference 검증 |

## 마이그레이션

```bash
# 미리보기 (파일 변경 없음)
claude-md-core migrate --root . --dry-run

# 마이그레이션 실행
claude-md-core migrate --root .
```

v1 파일은 모든 명령어에서 정상 동작합니다. v2 마이그레이션은 선택적입니다.
