Feature: Schema Validation
  As a developer maintaining CLAUDE.md files
  I want to validate that they follow the required schema
  So that they can be reliably used for code generation

  Background:
    Given a clean test directory

  Scenario: Missing Purpose fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims object
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Purpose"

  Scenario: Missing Exports fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Validates authentication tokens.

      ## Behavior
      - valid token → Claims object
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Exports"

  Scenario: Missing Behavior fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Behavior"

  Scenario: Valid TypeScript exports pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      ### Functions
      - `validateToken(token: string): Promise<Claims>`
      - `refreshToken(token: string, options?: RefreshOptions): Promise<TokenPair>`

      ## Behavior
      - valid token → Claims object
      - invalid token → InvalidTokenError

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid Python exports pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      ### Functions
      - `validate_token(token: str) -> Claims`
      - `refresh_token(token: str, options: RefreshOptions | None = None) -> TokenPair`

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid Go exports pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      ### Functions
      - `ValidateToken(token string) (Claims, error)`
      - `RefreshToken(token string, opts ...Option) (TokenPair, error)`

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid Rust exports pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      ### Functions
      - `validate_token(token: &str) -> Result<Claims, AuthError>`
      - `refresh_token(token: &str, options: Option<RefreshOptions>) -> Result<TokenPair, AuthError>`

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid Java exports pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      ### Methods
      - `Claims validateToken(String token) throws AuthException`
      - `TokenPair refreshToken(String token, RefreshOptions options)`

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Exports without parameter types produce warning
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `validateToken` - validates the token

      ## Behavior
      - token → result

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should have warnings
    And warning should mention "missing parameter types"

  Scenario: Behavior without arrow pattern fails
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `validateToken(token: string): boolean`

      ## Behavior
      - 토큰을 검증합니다
      - Validates the authentication token

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "input → output"

  Scenario: Valid behavior scenarios pass
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `process(input: string): Output`

      ## Behavior
      ### Normal cases
      - valid input → expected output
      - empty input → default output

      ### Error cases
      - invalid input → ValidationError
      - null input -> NullPointerException

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Enum-only exports pass validation
    Given CLAUDE.md with content:
      """
      # Config Module

      ## Purpose
      Configuration types and constants.

      ## Exports
      - `Status: Active | Inactive | Pending`

      ## Behavior
      - config load → Status value

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Variable-only exports pass validation
    Given CLAUDE.md with content:
      """
      # Constants Module

      ## Purpose
      Application constants.

      ## Exports
      - `MAX_RETRIES = 3`
      - `DEFAULT_TIMEOUT: Duration`

      ## Behavior
      - initialization → constants available

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Mixed enum, variable and function exports pass validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Authentication with mixed export types.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`
      - `Status: Active | Inactive | Pending`
      - `MAX_RETRIES = 3`

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Exports marked as None is valid
    Given CLAUDE.md with content:
      """
      # Internal Module

      ## Purpose
      Internal utilities with no public API.

      ## Exports
      None

      ## Behavior
      - Called internally by parent module → performs internal operations

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  # C-1: Forbidden reference validation (INV-1)
  Scenario: Dependencies with parent reference fails validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims object

      ## Dependencies
      - **Internal**: `../utils/crypto`

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "부모 디렉토리 참조 금지"

  Scenario: Dependencies with parent reference in subsection fails
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims object

      ## Dependencies

      ### Internal
      - `../utils/crypto`

      ### External
      - `jsonwebtoken@9.x`

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "부모 디렉토리 참조 금지"

  Scenario: Dependencies with child reference passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims object

      ## Dependencies
      - **Internal**: `./jwt/decoder`

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  # C-3: Table format support
  Scenario: Valid exports in table format pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports

      | Name | Signature | Description |
      |------|-----------|-------------|
      | `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |

      ## Behavior
      - valid token → Claims object

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid behavior in table format pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior

      | Input | Output |
      |-------|--------|
      | 유효한 JWT 토큰 | Claims 객체 반환 |
      | 만료된 토큰 | TokenExpiredError |

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Both exports and behavior in table format pass
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Exports

      | Name | Signature | Description |
      |------|-----------|-------------|
      | `validateToken` | `(token: string): Promise<Claims>` | JWT 토큰 검증 |

      ## Behavior

      | Input | Output |
      |-------|--------|
      | 유효한 JWT 토큰 | Claims 객체 반환 |
      | 만료된 토큰 | TokenExpiredError |

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema
    Then validation should pass
