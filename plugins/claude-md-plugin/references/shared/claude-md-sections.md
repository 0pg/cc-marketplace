# CLAUDE.md Sections Reference

> SSOT: `references/shared/schema-rules.yaml`

## 필수 섹션 (7개)

| 섹션 | "None" 허용 | 설명 |
|------|-------------|------|
| Purpose | No | 디렉토리의 책임 1-2문장 |
| Summary | No | 역할/책임/기능 1-2문장 요약 (dependency-graph CLI 노드 표시용) |
| Exports | Yes | public interface 시그니처 레벨. **Interface Catalog** 역할 |
| Behavior | Yes | 동작 시나리오 (input → output) |
| Contract | Yes | 함수별 사전/사후조건, 불변식 |
| Protocol | Yes | 상태 전이, 호출 순서 |
| Domain Context | Yes | compile 재현성 보장 맥락 (결정 근거, 제약, 호환성) |

## 선택 섹션 (3개)

| 섹션 | 조건 | 설명 |
|------|------|------|
| Structure | 하위 디렉토리/파일 있을 때 | 구조 설명 |
| Dependencies | 외부/내부 의존성 있을 때 | 의존성 목록 |
| Constraints | 제약사항 있을 때 | 규칙/제약 |

## "None" 표기 규칙

허용 값: `None`, `none`, `N/A`, `n/a`

Contract, Protocol, Domain Context 등 "None" 허용 섹션은 해당 내용이 없을 때 반드시 `None`을 명시합니다.

## v2 Exports 형식

v2에서는 `#### symbolName` heading으로 작성하여 cross-reference 앵커를 지원합니다:

```markdown
### Functions

#### validateToken
`validateToken(token: string): Promise<Claims>`

JWT 토큰을 검증하고 Claims를 추출합니다.
```

참조 형식: `path/CLAUDE.md#symbolName`

## v2 Behavior 구조 (UseCase)

```markdown
### Actors
- User: 역할 설명

### UC-1: UseCase Name
- Actor: User
- input → output
- Includes: UC-N
```

## 스키마 템플릿 참조

```bash
cat plugins/claude-md-plugin/templates/claude-md-schema.md
```
