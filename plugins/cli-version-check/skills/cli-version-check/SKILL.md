---
name: cli-version-check
description: |
  CLI 바이너리의 stale 여부를 점검합니다.
  소스 코드 변경 후 리빌드가 필요한 바이너리를 감지합니다.
  트리거: "/cli-version-check", "바이너리 체크", "리빌드 필요한지 확인"
allowed-tools: [Bash, Read]
---

# CLI Version Check

프로젝트의 CLI 바이너리가 소스 대비 최신 상태인지 점검합니다.

## Phase 1: 빌드 시스템 감지

프로젝트 루트에서 빌드 시스템을 감지합니다:
- `Cargo.toml` → Rust (cargo build)
- `go.mod` → Go (go build)
- `package.json` (bin 필드) → Node.js (npm/pnpm build)
- `Makefile` (BIN/TARGET 변수) → Make
- `pyproject.toml` / `setup.py` (scripts) → Python (pip install -e .)

## Phase 2: Staleness 비교

각 감지된 바이너리에 대해:
1. 바이너리 파일의 수정 시간(mtime)을 확인
2. 소스 디렉토리에서 바이너리보다 새로운 파일이 있는지 확인
3. stale / fresh / missing 상태를 판단

## Phase 3: 결과 보고 및 조치

- stale 바이너리가 있으면 리빌드 명령과 함께 알림
- 사용자에게 리빌드 실행 여부를 확인
- 승인 시 빌드 명령 실행
