---
name: dependency-graph
version: 1.0.0
description: (internal) 프로젝트 의존성 방향 그래프를 분석하고 경계 침범을 탐지
allowed-tools: [Bash, Read]
---

# Dependency Graph Skill

## 목적

프로젝트의 모듈 간 의존성을 분석하여 방향 그래프를 생성하고, 모듈 경계 침범을 탐지합니다.

**핵심 원칙: 경계 명확성 (INV-1)**
- **Exports(인터페이스) 참조**: 허용 - 다른 모듈의 공개 인터페이스 사용
- **내부 구현 직접 참조**: 금지 - 다른 모듈의 세부 구현에 의존

```
✓ auth → config.JWT_SECRET (Exports 참조)
✓ auth → utils/crypto.hashPassword (Exports 참조)
✗ auth → config 내부 로딩 로직 가정 (경계 침범)
✗ auth → utils/crypto 내부 알고리즘 직접 호출 (경계 침범)
```

## 입력

```
target_path: 분석 대상 경로 (기본: 프로젝트 루트)
```

## 출력

`.claude/dependency-graph.json` 파일 생성

```json
{
  "root": "/path/to/project",
  "analyzed_at": "2024-01-01T00:00:00Z",
  "nodes": [
    {
      "path": "src/auth",
      "has_claude_md": true,
      "summary": "인증 모듈. JWT 토큰 생성/검증/갱신 및 세션 관리 담당.",
      "exports": ["validateToken", "Claims"]
    }
  ],
  "edges": [
    {
      "from": "src/auth",
      "to": "src/config",
      "edge_type": "internal",
      "imported_symbols": [],
      "valid": true
    }
  ],
  "violations": [
    {
      "from": "src/auth",
      "to": "src/api",
      "violation_type": "missing-exports",
      "reason": "Module 'src/api' has no Exports defined in CLAUDE.md",
      "suggestion": "Add Exports section to src/api/CLAUDE.md with public interfaces"
    }
  ],
  "summary": {
    "total_nodes": 5,
    "total_edges": 8,
    "valid_edges": 7,
    "violations_count": 1
  }
}
```

## 워크플로우

### 1. CLI 빌드 확인

```bash
CLI_PATH="plugins/claude-md-plugin/core/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd plugins/claude-md-plugin/core && cargo build --release
fi
```

### 2. 의존성 그래프 생성

```bash
mkdir -p .claude
claude-md-core dependency-graph --root {target_path} --output .claude/dependency-graph.json
```

### 3. 결과 확인

```bash
if [ -f ".claude/dependency-graph.json" ]; then
    echo "Dependency graph completed"
else
    echo "Error: Dependency graph failed"
    exit 1
fi
```

## 결과 반환

```
---dependency-graph-result---
status: success
output_file: .claude/dependency-graph.json
total_nodes: {nodes 수}
total_edges: {edges 수}
violations_count: {violations 수}
---end-dependency-graph-result---
```

## 위반 유형

| 위반 유형 | 설명 | 예시 |
|----------|------|------|
| missing-exports | CLAUDE.md에 Exports 섹션이 없거나 비어있음 | `auth`가 `config` 참조, `config`에 Exports 없음 |
| boundary-violation | Exports에 없는 심볼 직접 참조 | `auth`가 `config` 내부 함수 사용 |

## 활용

### /spec에서의 활용

```python
# spec-agent Phase 2.5: 아키텍처 설계 분석
Skill("claude-md-plugin:dependency-graph")
graph = read_json(".claude/dependency-graph.json")

# 신규 모듈 배치 시 기존 의존성 구조 확인
# 경계 명확성 원칙 준수 여부 검증
# Exports를 통한 인터페이스 참조 가이드
```

### 출력 활용 예시

```markdown
## Module Integration Map (in IMPLEMENTS.md)

### 분석 결과
- 의존성 그래프: `.claude/dependency-graph.json` 참조
- 전체 모듈: 5개
- 유효 의존성: 8개
- 경계 침범: 0개

### `../config` → config/CLAUDE.md

#### Exports Used
- `PAYMENT_API_KEY: string` — 결제 API 키

#### Integration Context
결제 요청 시 인증에 사용.

### `../order` → order/CLAUDE.md

#### Exports Used
- `Order { id: string, items: OrderItem[], total: number }` — 주문 정보 타입

#### Integration Context
결제 처리 시 주문 금액 검증에 사용.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 대상 경로 없음 | 에러 메시지, 실패 반환 |
| CLAUDE.md 없음 | 해당 모듈 has_claude_md: false, exports: [] |
| CLI 빌드 실패 | cargo build 에러 출력, 실패 반환 |

## 지원 언어

| 언어 | 패턴 |
|------|------|
| TypeScript/JavaScript | `import ... from`, `require()` |
| Python | `from ... import`, `import ...` |
| Go | `import "..."` |
| Rust | `use crate::`, `use super::` |
| Java | `import ...` |
| Kotlin | `import ...` |

의존성 추출은 `code_analyzer` 모듈을 통해 수행됩니다.
