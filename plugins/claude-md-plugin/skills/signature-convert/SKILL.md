---
name: signature-convert
description: (internal) 함수 시그니처를 대상 언어 문법으로 변환
allowed-tools: [Bash]
---

# Signature Convert Skill

## 목적

언어 중립적인 함수 시그니처를 대상 프로그래밍 언어의 문법으로 변환합니다.
Rust CLI `claude-md-core convert-signature`를 래핑합니다.

## 지원 언어

| 언어 | CLI 값 | 특징 |
|------|--------|------|
| TypeScript | `typescript` 또는 `ts` | async/await, interface |
| Python | `python` 또는 `py` | async def, dataclass |
| Go | `go` 또는 `golang` | PascalCase, (Type, error) 패턴 |
| Rust | `rust` 또는 `rs` | Result<T, E>, snake_case |
| Java | `java` | CompletableFuture, throws |
| Kotlin | `kotlin` 또는 `kt` | suspend, data class |

## 입력

```
signature: 변환할 함수 시그니처 (언어 중립적)
target_lang: 대상 언어 (typescript, python, go, rust, java, kotlin)
```

## 출력

ConversionResult JSON:

```json
{
  "original_signature": "validateToken(token: string): Promise<Claims>",
  "converted_signature": "async def validate_token(token: str) -> Claims:",
  "target_language": "python",
  "function_name": "validate_token",
  "is_async": true
}
```

## 변환 예시

### TypeScript → Python

```
입력: validateToken(token: string): Promise<Claims>
출력: async def validate_token(token: str) -> Claims:
```

### TypeScript → Go

```
입력: validateToken(token: string): Promise<Claims>
출력: func ValidateToken(token string) (Claims, error)
```

### TypeScript → Rust

```
입력: validateToken(token: string): Promise<Claims>
출력: pub async fn validate_token(token: &str) -> Result<Claims, Error>
```

### TypeScript → Java

```
입력: validateToken(token: string): Promise<Claims>
출력: public CompletableFuture<Claims> validateToken(String token)
```

### TypeScript → Kotlin

```
입력: validateToken(token: string): Promise<Claims>
출력: suspend fun validateToken(token: String): Claims
```

## 네이밍 컨벤션 변환

| 원본 | TypeScript | Python | Go | Rust | Java | Kotlin |
|------|------------|--------|-----|------|------|--------|
| validateToken | validateToken | validate_token | ValidateToken | validate_token | validateToken | validateToken |
| TokenExpired | TokenExpired | token_expired | TokenExpired | token_expired | TokenExpired | TokenExpired |

## 타입 매핑

| 원본 | TypeScript | Python | Go | Rust | Java | Kotlin |
|------|------------|--------|-----|------|------|--------|
| string | string | str | string | String | String | String |
| number | number | int | int64 | i64 | long | Long |
| boolean | boolean | bool | bool | bool | boolean | Boolean |
| void | void | None | (없음) | () | void | Unit |

## 워크플로우

### 1. CLI 실행

```bash
claude-md-core convert-signature \
  --signature "{signature}" \
  --target-lang {target_lang}
```

### 2. 결과 파싱

JSON 출력을 파싱하여 `converted_signature` 추출

## 결과 반환

```
---signature-convert-result---
original: {원본 시그니처}
converted: {변환된 시그니처}
target_language: {대상 언어}
is_async: {true/false}
---end-signature-convert-result---
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 지원하지 않는 언어 | UnsupportedLanguage 에러 |
| 파싱 불가 시그니처 | ParseError 에러 |
