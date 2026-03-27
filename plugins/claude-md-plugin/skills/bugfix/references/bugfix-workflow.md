<!--
  bugfix-workflow.md
  Extracted from SKILL.md Steps 3 and 5 to reduce token overhead.
  Referenced by: SKILL.md, debugger-templates.md
-->

## Step 3: (Type C 전용) 모듈 탐색

기능 설명이 입력된 경우, 기술적 진단 대상을 먼저 특정:

**3.1. 전체 CLAUDE.md 인덱스 생성:**
```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")

TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}debug-scan-index.json"
```

**3.2. 의미적 매칭 & 신뢰도 분류:**

기능 설명의 키워드와 각 모듈의 `purpose` 매칭 후 결과 분류:

```
매칭 결과 분류:
- 확실 (1개, purpose 직접 일치) → 바로 3.3으로 진행
- 후보 다수 (2-3개) → Purpose 요약과 함께 AskUserQuestion으로 선택
- 매칭 실패 (0개) → Grep fallback:
    Grep: pattern="{keyword}" glob="**/*.{ts,py,go,rs}" head_limit=30
  → 여전히 없으면 AskUserQuestion으로 경로 직접 입력 요청
```

**3.3. 매칭된 모듈(최대 3개)에서 관련 테스트 탐색:**
```
Glob: {matched_dir}/**/*test*  또는  {matched_dir}/**/*spec*
```

**3.4. 관련 테스트 실행 → 실패 시 Type A/B 흐름 합류.**

**3.5. 전체 통과 → Requirements 교차 분석 (CLAUDE.md Requirements vs 실제 동작 비교).**

## Step 5: 사전 검증 (CLI) — 리스크 레벨 분류

CLAUDE.md가 존재할 때만 실행:

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
```

**5.1. 스키마 검증:**
```bash
$CLI_PATH validate-schema --file {claude_md_path}
```

**5.2. 미컴파일 변경 확인:**
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

**5.3. 리스크 레벨 분류:**

| 검증 결과 | 조건 | 리스크 | 대응 |
|-----------|------|--------|------|
| 스키마 FAIL (필수 섹션 누락) | Purpose/Requirements/Domain Context 오류 | **HIGH** | 차단 + AskUserQuestion 오버라이드 확인 |
| 스키마 FAIL (선택 섹션만) | 경고만 | **LOW** | 경고 후 계속 |
| 미컴파일: `untracked`/`no-source-code` | 소스코드 없음 | **HIGH** | 차단 + `/compile` 안내 |
| 미컴파일: `staged`/`modified`/`spec-newer` | 코드-스펙 불일치 | **MEDIUM** | 경고 + AskUserQuestion |
| 스키마 FAIL + 미컴파일 | 복합 | **HIGH** (에스컬레이션) | 차단 + 단계별 해결 안내 |
| 둘 다 PASS | 정상 | **NONE** | 그대로 진행 |

**HIGH 리스크 차단 시:**
```
AskUserQuestion: "사전 검증에서 HIGH 리스크가 발견되었습니다. {상세 설명}. 그래도 진행할까요?"
옵션: [오버라이드하고 진행, 먼저 해결 후 재시도]
```

**오버라이드 시 후속 처리:**
- debugger Task 호출 시 `risk_override: true` 플래그 전달
- debugger가 영향받는 계층 findings에 `confidence: LOW` 강제
