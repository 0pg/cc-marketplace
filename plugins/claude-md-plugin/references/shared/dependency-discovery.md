# Dependency Discovery via CLAUDE.md Tree

의존 모듈의 인터페이스가 필요할 때, **반드시 CLAUDE.md 트리를 먼저 탐색**합니다.

## 탐색 우선순위

| 우선순위 | 단계 | 탐색 대상 | 획득 정보 |
|----------|------|-----------|----------|
| 1 (필수) | 대상 CLAUDE.md Dependencies | 의존 모듈 경로 목록 | 어떤 모듈에 의존하는지 |
| 2 (필수) | 의존 모듈 CLAUDE.md Exports | 인터페이스 카탈로그 | 함수/타입/클래스 시그니처 |
| 3 (선택) | 의존 모듈 CLAUDE.md Behavior | 동작 이해 | 정상/에러 시나리오 |
| 4 (최후) | 실제 소스코드 | 구현 세부사항 | Exports만으로 불충분할 때만 |

## 실행 단계

1. spec.dependencies에서 내부 의존성 목록 추출
2. 각 의존성에 대해 `Read({dep.path}/CLAUDE.md)` → Exports 섹션 파싱
3. (선택) Behavior 섹션 파싱 (동작 이해 필요 시)

## 금지 사항

- 코드 먼저 탐색 후 CLAUDE.md 확인
- Exports 섹션 무시하고 바로 구현 파일 읽기
- 의존 모듈의 내부 구현 세부사항에 의존

**이유**: CLAUDE.md의 Exports는 **Interface Catalog**로 설계되었습니다.
코드 탐색보다 CLAUDE.md 탐색이 더 효율적이고, 캡슐화 원칙을 준수합니다.

## Symbol Cross-Reference Resolution (v2)

v2 크로스 레퍼런스(`path/CLAUDE.md#symbolName`)가 있을 때:

```bash
# 심볼 찾기 (go-to-definition)
claude-md-core symbol-index --root {project_root} --find symbolName

# 레퍼런스 찾기 (find-references)
claude-md-core symbol-index --root {project_root} --references "path/CLAUDE.md#symbolName"
```

- 크로스 레퍼런스 발견 시 해당 심볼의 시그니처로 import 문 생성에 활용
- 미해소 레퍼런스는 경고 로그
