# CLI Version Check Plugin

CLI 바이너리가 소스 대비 stale 상태인지 감지하는 플러그인입니다.

## 동작 방식

### SessionStart Hook
세션 시작 시 자동으로 실행되어 stale 바이너리를 감지합니다.

```
1. 빌드 시스템 감지 (Cargo, Go, Node, Make, Python)
2. 바이너리 mtime vs 소스 파일 mtime 비교
3. stale 바이너리 발견 시 경고 출력
```

### `/cli-version-check` Skill
수동으로 바이너리 상태를 점검하고, 필요시 리빌드를 실행합니다.

## 지원 빌드 시스템

| 빌드 시스템 | 감지 파일 | 바이너리 위치 |
|------------|----------|-------------|
| Cargo | `Cargo.toml` | `target/release/`, `target/debug/` |
| Go | `go.mod` | `$GOPATH/bin/`, `./bin/`, 프로젝트 루트 |
| Node.js | `package.json` (bin) | bin 필드 경로 |
| Make | `Makefile` | BIN/TARGET 변수 |
| Python | `pyproject.toml`, `setup.py` | `.venv/bin/` |

## 의존성

- `jq`: JSON 파싱 (없으면 graceful skip)
- `find`, `stat`: 파일 시스템 비교
