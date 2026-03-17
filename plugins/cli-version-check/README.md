# cli-version-check

CLI 바이너리의 stale 여부를 자동 감지하여 리빌드가 필요할 때 알려주는 플러그인입니다.

## 문제

소스 코드를 수정했지만 CLI 바이너리를 리빌드하지 않으면, 이전 버전의 바이너리를 계속 사용하게 됩니다.
이 플러그인은 세션 시작 시 바이너리와 소스 파일의 수정 시간을 비교하여 리빌드 필요 여부를 알려줍니다.

## 설치

```bash
# Claude Code 플러그인으로 설치
claude plugin add ./plugins/cli-version-check
```

## 기능

- **자동 감지**: 세션 시작 시 stale 바이너리 자동 감지
- **다중 빌드 시스템**: Cargo, Go, Node.js, Make, Python 지원
- **수동 점검**: `/cli-version-check`로 수동 실행 가능
- **빌드 명령 안내**: 각 빌드 시스템에 맞는 리빌드 명령 제공

## 지원 빌드 시스템

- **Rust** (Cargo): `Cargo.toml` → `target/release/`, `target/debug/`
- **Go**: `go.mod` → `$GOPATH/bin/`, `./bin/`
- **Node.js**: `package.json` bin 필드 → 지정된 경로
- **Make**: `Makefile` BIN/TARGET 변수 → 지정된 경로
- **Python**: `pyproject.toml` / `setup.py` scripts → `.venv/bin/`
