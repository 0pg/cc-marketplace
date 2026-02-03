# auth

## Purpose
User authentication module for JWT token validation and generation.

## Exports

### Functions
- `validateToken(token: string): Promise<Claims>`
- `generateToken(userId: string, role: Role): string`

### Types
- `Claims { userId: string, role: Role, exp: number }`
- `TokenConfig { secret: string, expiresIn: number }`

### Classes
- `TokenManager(secret: string, algorithm?: string)`

## Dependencies
- external: jsonwebtoken@9.0.0
- internal: ./types

## Behavior

### Success Cases
- valid JWT token → Claims object with userId and role
- generateToken with valid params → JWT string

### Error Cases
- expired token → TokenExpiredError
- invalid signature → InvalidTokenError
- malformed token → MalformedTokenError

## Contract

### validateToken
- **Preconditions**: token must be non-empty string
- **Postconditions**: returns Claims with valid userId
- **Throws**: InvalidTokenError if token is malformed
