# /impl Workflow Detail

## scan-claude-md 호출 패턴

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

# CLI로 기존 CLAUDE.md 파일의 경량 인덱스 생성
mkdir -p .claude/extract-results
$CLI_PATH scan-claude-md --root {project_root} --output .claude/extract-results/claude-md-index.json
```

인덱스 출력 형식:
```json
{
  "root": "/path/to/project",
  "entries": [
    {
      "dir": "src/auth",
      "purpose": "JWT 토큰 검증 인증 모듈",
      "export_names": ["validateToken", "Claims", "TokenError"]
    }
  ]
}
```

## impl agent 워크플로우 (Phase 0~7)

### 0. ⭐ Scope Assessment (NEW)
- 완성도 분류: high / medium / low
- 멀티 모듈 감지: single-module / multi-module
- multi-module → AskUserQuestion (분해/도메인 그룹/단일유지)

### 1. 요구사항 분석
- 자연어에서 Purpose, Exports, Behaviors, Contracts 추출
- User Story, Feature 목록, 기능 요청 형태 지원

### 1.5. 의존성 탐색 (dep-explorer)
- scan 인덱스(~6KB) 로드 → purpose + export_names로 관련 모듈 판단
- LLM 판단 기반 (programmatic 필터링 아님)
- 관련 모듈만 선별적 Read (3-5개 수준)

### 2. ⭐ Tiered Clarification (UPDATED)
- Round 1 — Tier 1(범위): PURPOSE, LOCATION (completeness=low일 때)
- Round 2 — Tier 2(인터페이스)+3(제약): EXPORTS, BEHAVIOR, CONTRACT, DOMAIN_CONTEXT
- 최대 2라운드, 라운드당 최대 4질문
- completeness=high이면 Phase 2 건너뛰기 (경로 미지정 시 LOCATION만 질문)

### 3. 대상 위치 결정
- 명시적 경로 > 모듈명 추론 > 사용자 선택
- 기존 디렉토리 존재 시 update, 없으면 create

### 4. 기존 CLAUDE.md 존재시 병합
- Purpose: 기존 유지 또는 확장 (사용자 선택)
- Exports: 이름 기준 병합 (기존 유지 + 신규 추가)
- Behavior: 시나리오 추가 (중복 제거)
- Contract: 함수명 기준 병합
- Dependencies: Union

### 5. CLAUDE.md 생성
- 필수 6개 섹션: Purpose, Exports, Behavior, Contract, Protocol, Domain Context
- Contract/Protocol/Domain Context는 "None" 허용

### 5.5. IMPLEMENTS.md Planning Section 생성
- Dependencies Direction: CLAUDE.md 경로로 resolve된 의존성
- Implementation Approach: 구현 전략과 대안
- Technology Choices: 기술 선택 근거
- Implementation Section은 "(To be filled by /compile)" 플레이스홀더

### 6. 스키마 검증 (1회)
- claude-md-core validate-schema CLI 호출
- 실패 시 경고와 함께 이슈 보고

### 6.5. ⭐ Plan Preview (NEW)
- 생성 계획 요약 제시 (경로, Purpose, Exports, Behaviors, Dependencies)
- AskUserQuestion: 승인 / 범위조정 / 위치변경 / 취소
- 수정 요청 시 → Phase 5~6.5 재실행 (최대 1회 루프백)
- 승인된 경우만 Phase 7로 진행

### 7. 최종 저장
- 승인된 경우에만 CLAUDE.md + IMPLEMENTS.md 파일 저장

## Planning Section 생성 로직

### Dependencies Direction
```markdown
### External
- `jsonwebtoken@9.0.0`: JWT 검증 (선택 이유: 성숙한 라이브러리)

### Internal
- `utils/crypto/CLAUDE.md`: hashPassword, verifyPassword (해시 유틸리티)
```

의존성 탐색 우선순위:
1. Internal (기존 CLAUDE.md) - scan 인덱스 매칭
2. Existing External (프로젝트에 이미 선언된 의존성)
3. New External (AskUserQuestion으로 승인 필수)
