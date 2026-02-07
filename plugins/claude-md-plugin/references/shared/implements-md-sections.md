# IMPLEMENTS.md Sections Reference

> SSOT: `templates/implements-md-schema.md`

## Planning Section (`/spec`이 업데이트)

| 섹션 | 필수 | "None" 허용 | 설명 |
|------|------|-------------|------|
| Architecture Decisions | Yes | Yes | 모듈 배치, 인터페이스 설계, 의존성 방향 결정 |
| Dependencies Direction | Yes | No | External/Internal 의존성 위치와 용도 |
| Implementation Approach | Yes | No | 구현 전략 + 고려했으나 선택하지 않은 대안 |
| Technology Choices | Yes | Yes | 기술 선택 근거 (테이블 형식) |

## Implementation Section (`/compile`이 업데이트)

| 섹션 | 필수 | 설명 |
|------|------|------|
| Algorithm | 조건부 | 복잡하거나 비직관적인 로직만 |
| Key Constants | 조건부 | 도메인 의미 있는 상수만 |
| Error Handling | Yes ("None" 허용) | 에러 유형별 처리 전략 |
| State Management | Yes ("None" 허용) | 초기 상태, 저장, 정리 |
| Implementation Guide | 조건부 | 다른 세션 참고용 구현 가이드 |

## 명령어별 업데이트 책임

```
/spec      → CLAUDE.md + IMPLEMENTS.md Planning Section
/compile   → IMPLEMENTS.md Implementation Section
/decompile → CLAUDE.md + IMPLEMENTS.md 전체
```

## 스키마 템플릿 참조

```bash
cat plugins/claude-md-plugin/templates/implements-md-schema.md
```
