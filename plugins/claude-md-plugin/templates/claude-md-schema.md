# CLAUDE.md Schema Template

이 템플릿은 CLAUDE.md 파일의 표준 구조를 정의합니다.

## 필수 섹션

### 1. Purpose (필수)
디렉토리의 책임을 1-2문장으로 명시합니다.

```markdown
## Purpose
이 모듈은 사용자 인증을 담당합니다.
```

### 2. Structure (조건부 필수)
하위 디렉토리나 파일이 있는 경우 필수입니다.

```markdown
## Structure
- jwt/: JWT 토큰 처리 (상세는 jwt/CLAUDE.md 참조)
- session.ts: 세션 관리 로직
- types.ts: 인증 관련 타입 정의
```

### 3. Exports (필수)
모듈의 public interface를 **시그니처 레벨**로 명시합니다.

#### 형식 규칙
- 함수/메서드: `Name(params) ReturnType` 형태
- 파라미터 타입과 반환 타입 필수
- 언어별 관용 표현 허용

#### 언어별 예시

**TypeScript:**
```markdown
### Functions
- `validateToken(token: string): Promise<Claims>`
- `refreshToken(token: string, options?: RefreshOptions): Promise<TokenPair>`

### Types
- `Claims { userId: string, role: Role, exp: number }`
- `TokenPair { accessToken: string, refreshToken: string }`
```

**Python:**
```markdown
### Functions
- `validate_token(token: str) -> Claims`
- `refresh_token(token: str, options: RefreshOptions | None = None) -> TokenPair`

### Classes
- `TokenManager(secret: str, algorithm: str = "HS256")`
```

**Go:**
```markdown
### Functions
- `ValidateToken(token string) (Claims, error)`
- `RefreshToken(token string, opts ...Option) (TokenPair, error)`

### Types
- `Claims struct { UserID string; Role Role; Exp int64 }`
```

**Rust:**
```markdown
### Functions
- `validate_token(token: &str) -> Result<Claims, AuthError>`
- `refresh_token(token: &str, options: Option<RefreshOptions>) -> Result<TokenPair, AuthError>`

### Structs
- `Claims { user_id: String, role: Role, exp: i64 }`
```

**Java/Kotlin:**
```markdown
### Methods
- `Claims validateToken(String token) throws AuthException`
- `TokenPair refreshToken(String token, RefreshOptions options)`

### Classes
- `TokenManager(String secret, Algorithm algorithm)`
```

### 4. Dependencies (조건부)
외부 의존성이 있는 경우 명시합니다.

```markdown
## Dependencies
- external: jsonwebtoken@9.0.0
- internal: ../utils/crypto
```

### 5. Behavior (필수)
동작을 **시나리오 레벨** (input → output)로 명시합니다.

```markdown
## Behavior

### 정상 케이스
- 유효한 토큰 → Claims 객체 반환
- 만료된 토큰 + refresh 옵션 → 새 토큰 쌍 반환

### 에러 케이스
- 잘못된 형식의 토큰 → InvalidTokenError
- 만료된 토큰 (refresh 없음) → TokenExpiredError
- 위조된 토큰 → SignatureVerificationError
```

### 6. Constraints (선택)
지켜야 할 규칙이나 제약사항입니다.

```markdown
## Constraints
- 토큰 만료 시간은 최대 24시간
- refresh token은 secure storage에만 저장
- 동시 세션은 최대 5개
```

## 검증 규칙

### 필수 섹션 검증
- Purpose: 반드시 존재
- Exports: 반드시 존재 (public interface가 없는 경우도 "None" 명시)
- Behavior: 반드시 존재

### Exports 형식 검증
```regex
# 함수 패턴: Name(params) ReturnType 형태
^[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*[:→\->]?\s*.+$
```

유효 예시:
- `validateToken(token: string): Promise<Claims>` ✓
- `validate_token(token: str) -> Claims` ✓
- `ValidateToken(token string) (Claims, error)` ✓

무효 예시:
- `validateToken` (파라미터 없음) ✗
- `validate token` (공백 포함) ✗

### Behavior 형식 검증
```regex
# 시나리오 패턴: input → output 형태
.+\s*[→\->]\s*.+
```

유효 예시:
- `유효한 토큰 → Claims 객체` ✓
- `invalid input -> specific error` ✓

무효 예시:
- `토큰을 검증합니다` (시나리오가 아님) ✗

## 참조 규칙

### 허용
- 부모 → 자식: 자식 디렉토리 참조 가능

### 금지
- 자식 → 부모: 부모 디렉토리 참조 불가
- 형제 ↔ 형제: 형제 디렉토리 상호 참조 불가

```markdown
# src/CLAUDE.md에서
## Structure
- auth/: 인증 모듈 (상세는 auth/CLAUDE.md 참조) ✓

# src/auth/CLAUDE.md에서
## Dependencies
- ../api: (부모 참조 - 금지) ✗
- ../utils: (형제 참조 - 금지) ✗
```
