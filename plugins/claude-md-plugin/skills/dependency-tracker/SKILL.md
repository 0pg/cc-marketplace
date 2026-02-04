---
name: dependency-tracker
version: 1.1.0
description: |
  Tracks module dependencies and analyzes impact of changes on dependent modules.
  Invoked by compile skill after interface-diff detects breaking changes to recommend recompilation targets.
allowed-tools: [Read, Write, Glob, Grep]
---

# Dependency Tracker Skill

## 목적

모든 CLAUDE.md의 Dependencies 섹션을 파싱하여 의존 그래프를 구축합니다.
변경된 모듈의 의존자(dependents)를 찾아 재컴파일 대상을 추천합니다.

## 입력

```
target_path: 분석 대상 경로 (기본값: .)
changed_modules: 변경된 모듈 경로 목록
output_name: 출력 파일명 (기본값: dependency)
```

## 출력

`.claude/incremental/{output_name}-impact.json` 파일 생성

```json
{
  "dependency_graph": {
    "src/auth": {
      "depends_on": ["src/utils/crypto", "src/config"],
      "depended_by": ["src/api", "src/middleware"]
    },
    "src/api": {
      "depends_on": ["src/auth", "src/db"],
      "depended_by": ["src/routes"]
    }
  },
  "impact_analysis": [
    {
      "changed_module": "src/auth",
      "dependents": [
        {
          "path": "src/api",
          "imports": ["validateToken", "Claims"],
          "impact_level": "high"
        },
        {
          "path": "src/middleware",
          "imports": ["validateToken"],
          "impact_level": "high"
        }
      ],
      "transitive_dependents": ["src/routes"]
    }
  ],
  "recommendation": {
    "recompile_required": ["src/api", "src/middleware"],
    "recompile_suggested": ["src/routes"],
    "commands": [
      "/compile --path src/api",
      "/compile --path src/middleware"
    ]
  },
  "summary": {
    "total_modules": 5,
    "changed_modules": 1,
    "direct_impact": 2,
    "transitive_impact": 1
  }
}
```

## 의존성 추출 방법

### CLAUDE.md Dependencies 섹션 파싱

```markdown
## Dependencies

### Internal
- `../utils/crypto` - 암호화 유틸리티
- `../config` - 설정 관리

### External
- `jsonwebtoken` - JWT 처리
```

### 소스 코드 import 분석 (보조)

```typescript
// TypeScript/JavaScript
import { validateToken } from '../auth'
import type { Claims } from '../auth/types'

// Python
from ..auth import validate_token
from ..auth.types import Claims
```

## 워크플로우

### Step 1: 모든 CLAUDE.md 검색 및 파싱

```
Glob: **/CLAUDE.md (root 제외)

for each CLAUDE.md:
  Parse Dependencies 섹션
  → depends_on 목록 추출
```

### Step 2: 의존 그래프 구축

```
graph = {}
for each module:
  graph[module] = {
    depends_on: [...],
    depended_by: []
  }

# 역방향 관계 구축
for each module:
  for dep in module.depends_on:
    graph[dep].depended_by.append(module)
```

### Step 3: 영향 분석

```
for each changed_module:
  direct_dependents = graph[changed_module].depended_by

  # 전이적 의존자 (BFS)
  transitive = bfs(graph, direct_dependents)
```

### Step 4: import 상세 분석

```
for each dependent:
  Read CLAUDE.md or source
  Extract specific imports from changed_module
  → import 목록 및 영향도 판정
```

### Step 5: 추천 생성

```
recompile_required = direct_dependents with breaking changes
recompile_suggested = transitive_dependents
commands = generate /compile commands
```

### Step 6: JSON 결과 생성 및 저장

```
Write → .claude/incremental/{output_name}-impact.json
```

## 결과 반환

```
---dependency-tracker-result---
output_file: .claude/incremental/{output_name}-impact.json
status: success
total_modules: {전체 모듈 수}
changed_modules: {변경된 모듈 수}
direct_impact: {직접 영향받는 모듈 수}
transitive_impact: {간접 영향받는 모듈 수}
recompile_required: [{재컴파일 필수 모듈}]
recompile_suggested: [{재컴파일 권장 모듈}]
---end-dependency-tracker-result---
```

## 영향도 (Impact Level)

| 레벨 | 의미 | 조건 |
|------|------|------|
| `high` | 재컴파일 필수 | breaking change 있는 export를 import |
| `medium` | 재컴파일 권장 | non-breaking change 있는 export를 import |
| `low` | 확인 권장 | 전이적 의존성으로만 연결 |

## 예시

### 시나리오: src/auth 변경

**의존 그래프:**
```
src/config ← src/auth ← src/api ← src/routes
                     ← src/middleware
```

**입력:**
```json
{
  "changed_modules": ["src/auth"]
}
```

**결과:**
```json
{
  "impact_analysis": [{
    "changed_module": "src/auth",
    "dependents": [
      {"path": "src/api", "imports": ["validateToken", "Claims"]},
      {"path": "src/middleware", "imports": ["validateToken"]}
    ],
    "transitive_dependents": ["src/routes"]
  }],
  "recommendation": {
    "recompile_required": ["src/api", "src/middleware"],
    "recompile_suggested": ["src/routes"],
    "commands": [
      "/compile --path src/api",
      "/compile --path src/middleware"
    ]
  }
}
```

## 순환 의존성 처리

순환 의존성 감지 시:

```json
{
  "circular_dependencies": [
    ["src/a", "src/b", "src/a"]
  ],
  "warning": "Circular dependency detected. Consider refactoring."
}
```

- BFS에서 이미 방문한 노드는 스킵
- 순환 감지 시 경고 포함

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | 빈 결과 반환 |
| Dependencies 섹션 없음 | depends_on을 빈 배열로 처리 |
| 순환 의존성 | 경고 포함, 분석은 계속 |
| 경로 해석 실패 | 해당 의존성 스킵, 경고 로그 |

## 참고

- 이 스킬은 `interface-diff`와 연계하여 사용됩니다.
- breaking change가 있는 모듈의 의존자만 재컴파일 필수로 표시됩니다.
- 전이적 의존자는 명시적으로 재컴파일하지 않아도 되지만, 확인을 권장합니다.
