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
- [x] Skills: tree-parse, boundary-resolve, code-analyze, schema-validate
- [x] /decompile 명령어

### v0.18.0 - Compile/Decompile 패러다임
- [x] `/init` → `/decompile` 리네임
- [x] `/generate` → `/compile` 리네임
- [x] Core Philosophy 문서화 (CLAUDE.md = Source Code)

### v0.19.0 - 아키텍처 단순화
- [x] Agent 통합: spec-clarifier + spec-writer → spec-agent
- [x] decompiler 간소화: draft-generate Skill 인라인화
- [x] signature-convert 제거: LLM 직접 해석으로 대체
- [x] 재시도 정책 변경: 스키마 검증 1회, 테스트 3회
- [x] 삭제된 파일: 10개 (3,124줄), 추가된 파일: 1개 (234줄)

---

## 다음 마일스톤

### Phase 1: Generator Agent ✅ 완료

CLAUDE.md를 읽고 소스코드를 생성하는 Agent.

```
Input: CLAUDE.md
Output: 소스코드 파일들
```

#### 구현 범위
- [x] compiler Agent 정의
- [x] /compile 명령어 (사용자 진입점)
- [x] TDD 워크플로우 (RED → GREEN → REFACTOR)
- [x] LLM 기반 시그니처 해석 (signature-convert 제거)

#### 생성 규칙
- Exports 섹션의 시그니처를 정확히 구현
- Behavior 섹션의 시나리오를 만족하는 로직 구현
- Dependencies 섹션 기반 import 생성

---

### Phase 2: Validator Agent ✅ 완료 (단순화)

CLAUDE.md의 품질을 검증하는 Agent.

```
Input: CLAUDE.md + 현재 소스코드
Output: Pass/Fail + 불일치 리포트
```

#### 구현된 검증 방식

**v0.19.0에서 단순화됨**: 원래 계획된 복잡한 3단계 검증 대신, 빠른 sanity check 방식 채택

| Validator | 역할 | 구현 상태 |
|-----------|------|----------|
| drift-validator | CLAUDE.md와 코드 일치 검증 | [x] 완료 |
| export-validator | Export 존재 여부 확인 | [x] 완료 (이전 이름: reproducibility-validator) |

#### 검증 범위
- [x] /validate 명령어 (사용자 진입점)
- [x] 구조 검증 (Structure drift)
- [x] Export 존재 검증 (grep 기반)
- [x] 불일치 리포트 생성

**미구현 (향후 고려)**:
- [ ] 인터페이스 검증 (시그니처 정확 일치)
- [ ] 동작 검증 (Behavior 시나리오 테스트)

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
