# claude-md-plugin Roadmap

## 목표

**CLAUDE.md만 읽고 Claude Code가 동일한 구조와 기능의 소스코드를 80% 이상의 확률로 재현할 수 있는 문서 체계 구축**

---

## 완료된 마일스톤

### v0.6.0 - Extractor 기반 구축
- [x] Core Engine (Rust CLI)
  - tree_parser: 디렉토리 스캔, CLAUDE.md 필요 판정
  - boundary_resolver: 바운더리 결정, 참조 검증
  - schema_validator: 스키마 검증
- [x] Extractor Agent: 소스코드 → CLAUDE.md 추출
- [x] Skills: tree-parse, boundary-resolve, code-analyze, draft-generate, schema-validate
- [x] /extract 명령어

---

## 다음 마일스톤

### Phase 1: Generator Agent

CLAUDE.md를 읽고 소스코드를 생성하는 Agent.

```
Input: CLAUDE.md
Output: 소스코드 파일들
```

#### 구현 범위
- [ ] generator Agent 정의
- [ ] /generate 명령어 (사용자 진입점)
- [ ] 언어별 코드 생성 템플릿
- [ ] 생성 결과 리포트

#### 생성 규칙
- Exports 섹션의 시그니처를 정확히 구현
- Behavior 섹션의 시나리오를 만족하는 로직 구현
- Dependencies 섹션 기반 import 생성

---

### Phase 2: Validator Agent

CLAUDE.md의 품질을 검증하는 Agent. 해당 CLAUDE.md만 읽고 소스코드를 재생성했을 때 기존과 동일한 결과가 나오는지 확인.

```
Input: CLAUDE.md + 현재 소스코드
Output: Pass/Fail + 불일치 리포트
```

#### 검증 프로세스

```
CLAUDE.md ──────┬──────────────────────────────────►
     │          │
     │     [Generator Agent]
     │          │
     ▼          ▼
현재 소스코드   생성된 소스코드
(ground truth)  (prediction)
     │          │
     └────┬─────┘
          │
          ▼
   [Validator Agent]
          │
          ▼
   ┌─────────────────────┐
   │ 1. 구조 검증         │ 파일명/디렉터리명 일치 여부
   │ 2. 인터페이스 검증   │ Export 시그니처 일치 여부
   │ 3. 동작 검증         │ Behavior 시나리오 충족 여부
   └─────────────────────┘
          │
          ▼
   Pass (≥80%) / Fail
```

#### 검증 순서
1. **Leaf 노드부터 검증**: 의존성이 없는 최하위 디렉토리부터 시작
2. **각 노드는 자기 바운더리만 검증**: 직접 소유한 파일만 검증 대상
3. **하위 디렉토리는 존재 여부만 확인**: 내부 검증은 해당 CLAUDE.md 책임

#### 구현 범위
- [ ] validator Agent 정의
- [ ] /validate 명령어 (사용자 진입점)
- [ ] 구조 검증 로직 (파일명, 디렉토리명)
- [ ] 인터페이스 검증 로직 (Export 시그니처 비교)
- [ ] 동작 검증 로직 (Behavior 시나리오 테스트)
- [ ] 불일치 리포트 생성

---

### Phase 3: CI 통합

#### 구현 범위
- [ ] Pre-commit hook: 소스코드 변경 시 CLAUDE.md 동기화 검사
- [ ] GitHub Action: PR에서 자동 validation
- [ ] 불일치 시 자동 리포트 생성

---

## 성공 지표

| 지표 | 목표 | 측정 방법 |
|------|------|----------|
| **재현율** | ≥80% | Generator가 생성한 코드와 실제 코드의 구조/기능 일치율 |
| **문서-코드 동기화** | 100% | CLAUDE.md 변경 없이 소스코드만 변경 시 Validation 실패 |
| **온보딩 시간** | 측정 가능 | 새 팀원이 CLAUDE.md만으로 모듈 이해 가능 여부 |

---

## 불변식 (구현 시 준수)

### INV-3: 재현 가능성
```
∀ node ∈ CLAUDE.md tree:
  P(generate(node.claude_md) ≈ node.current_files) ≥ 0.8
```

### INV-4: Export 명세 완전성
```
∀ node ∈ CLAUDE.md tree:
  node.exports = complete_public_interface(node.direct_files)
```

---

## 참고

- 현재 구현 상세: [CLAUDE.md](./CLAUDE.md)
- 사용법: [README.md](./README.md)
