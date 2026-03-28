# Compiler Agent - Detailed Workflow Reference
<!--
  v9: Session file input model. SKILL extracts specs into session file, compiler reads it.
  Composes superpowers:test-driven-development for RED-GREEN-REFACTOR discipline.
  Domain-specific rules (Constraints→test mapping, Convention application) remain here.
-->

## 워크플로우

### Phase 1: 세션 파일 읽기

세션 파일(SKILL이 생성)을 Read합니다. 모든 스펙이 이미 추출되어 있습니다:

- **Requirements**: 비즈니스 요구사항 (구현 기준)
- **Constraints**: 정밀한 입출력 계약 (테스트 원천)
- **Technical Context**: 기술 선택 + 근거 (상수/설정 원천)
- **Conventions**: 계층 해소 완료 (SKILL에서 module > project > general 처리)
- **Dependencies**: 의존 모듈 인터페이스
- **Verification Contract**: 완료 조건

모호한 스펙이 있으면 `## Origin` 섹션의 원본 경로를 Read하여 확인합니다.

#### 1.1 스펙 → 코드 변환 규칙

| 세션 파일 요소 | 생성 코드 |
|---------------|----------|
| Constraints `token.expiresAt <= now + 7d` | `const MAX_TOKEN_EXPIRY_DAYS = 7;` + 검증 로직 |
| Constraints `input.encoding == UTF-8` | 인코딩 검증 guard clause |
| Technical Context `IdP SLA 500ms, timeout = SLA × 4` | `const TIMEOUT_MS = 2000; // Based on IdP SLA` |
| Domain Context `PCI-DSS 준수` | `// PCI-DSS compliance` 주석 |
| Requirements (Constraints 없을 때 fallback) | 고수준 검증 로직 |

### Phase 2: 테스트 생성 (RED)

> RED-GREEN-REFACTOR 규율은 superpowers:test-driven-development를 따릅니다.
> 이 Phase는 **도메인 특화 테스트 매핑 규칙**만 정의합니다.

세션 파일의 Constraints에서 테스트를 생성합니다. Constraints가 없으면 Requirements에서 fallback.

#### 2.1 Constraints → 테스트 매핑

1. **수치 제한** → 경계값 테스트
   - `"token.expiresAt <= now + 7d"` → `test: 7일 OK, 8일 실패`
2. **형식 제약** → 유효/무효 입력 테스트
   - `"input.encoding == UTF-8"` → `test: UTF-8 OK, non-UTF-8 실패`
3. **비즈니스 규칙** → 규칙 준수/위반 시나리오
   - `"throws DuplicateError when exists"` → `test: 중복 시 에러`

#### 2.2 Technical Context → 경계값/상수 추출

| Technical Context | 추출 값 | 테스트 활용 |
|-------------------|---------|-----------|
| `IdP SLA 500ms, 타임아웃 = SLA × 4` | `2000` | 타임아웃 경계 테스트 |
| `PCI-DSS 7일 만료` | `7` | 만료 경계 테스트 |

#### 2.3 기존 소스 참조 (overwrite 모드)

`overwrite` 모드에서 기존 소스가 있으면:
```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
$CLI_PATH analyze-code --path {target_dir}
```
인터페이스를 발견하여 테스트 호환성 보장.

#### 2.4 테스트 파일 생성 + RED 검증

언어별 테스트 프레임워크에 맞는 테스트 파일을 Write합니다.
RED 검증은 superpowers:tdd "Verify RED" 절차를 따릅니다:
```bash
{test_command}
# 실패 확인 필수. 통과하면 테스트 수정. 에러(실패 아닌)면 에러 수정 후 재실행.
```

### Phase 3: GREEN Phase

superpowers:tdd "Minimal Code" + "Verify GREEN" 절차를 따릅니다.
도메인 규칙:
- Constraints assertion 변경 금지. Import 오타만 수정.
- 최대 3회 재시도. 3회 후에도 실패하면 경고 기록.
- 타입/인터페이스 파일 → 메인 구현 파일 순서로 생성.

### Phase 4: REFACTOR Phase

superpowers:tdd "Clean Up" + "stay green" 절차를 따릅니다.
도메인 규칙:
- 세션 파일의 Conventions 섹션(이미 계층 해소됨) 적용
- 회귀 테스트 실행. 실패 시 롤백.

### Phase 5: 파일 충돌 처리

생성된 각 파일에 대해 대상 경로에 파일이 이미 존재하는지 확인합니다:
- 세션 파일의 `conflict` 모드가 "skip"이면 기존 파일을 유지하고 건너뜁니다.
- "overwrite"이면 기존 파일을 덮어씁니다.
- 존재하지 않으면 새 파일을 생성합니다.

### Phase 6: 결과 반환

다음 구조의 결과 JSON을 생성하여 파일에 저장합니다:

```json
{
  "target_dir": "{target_dir}",
  "detected_language": "{detected_language}",
  "generated_files": ["{written_files}"],
  "skipped_files": ["{skipped_files}"],
  "tests": { "total": "{total}", "passed": "{passed}", "failed": "{failed}" },
  "status": "success | warning"
}
```

결과 블록:
```
---compiler-result---
result_file: {result_file}
status: {status}
generated_files: {written_files}
skipped_files: {skipped_files}
tests_passed: {passed}
tests_failed: {failed}
---end-compiler-result---
```
