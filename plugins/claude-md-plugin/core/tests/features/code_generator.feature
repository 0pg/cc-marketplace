Feature: Code Generator
  As a generator agent
  I want to generate source code from CLAUDE.md specifications
  So that the specification becomes the source of truth for the codebase

  Background:
    Given the code generator uses TDD workflow internally
    And the generator supports 6 target languages

  # =============================================================================
  # Language Detection
  # =============================================================================

  Scenario: Auto-detect TypeScript from existing files
    Given a directory "src/auth" with files:
      | file          |
      | index.ts      |
      | types.ts      |
    And a CLAUDE.md file at "src/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "TypeScript"

  Scenario: Auto-detect Python from existing files
    Given a directory "src/auth" with files:
      | file          |
      | __init__.py   |
      | auth.py       |
    And a CLAUDE.md file at "src/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "Python"

  Scenario: Auto-detect Go from existing files
    Given a directory "pkg/auth" with files:
      | file       |
      | token.go   |
      | claims.go  |
    And a CLAUDE.md file at "pkg/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "Go"

  Scenario: Auto-detect Rust from existing files
    Given a directory "src/auth" with files:
      | file    |
      | mod.rs  |
      | lib.rs  |
    And a CLAUDE.md file at "src/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "Rust"

  Scenario: Auto-detect Java from existing files
    Given a directory "src/main/java/auth" with files:
      | file              |
      | TokenService.java |
      | Claims.java       |
    And a CLAUDE.md file at "src/main/java/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "Java"

  Scenario: Auto-detect Kotlin from existing files
    Given a directory "src/main/kotlin/auth" with files:
      | file              |
      | TokenService.kt   |
      | Claims.kt         |
    And a CLAUDE.md file at "src/main/kotlin/auth/CLAUDE.md"
    When I detect the target language
    Then the detected language should be "Kotlin"

  Scenario: Detect from parent CLAUDE.md when no source files
    Given an empty directory "src/auth/jwt"
    And a parent CLAUDE.md at "src/auth/CLAUDE.md" specifying language "TypeScript"
    When I detect the target language for "src/auth/jwt"
    Then the detected language should be "TypeScript"

  Scenario: Prompt user when detection fails
    Given an empty directory "src/new-module"
    And no parent CLAUDE.md with language information
    When I detect the target language
    Then the generator should ask user for language preference

  # =============================================================================
  # TypeScript Code Generation
  # =============================================================================

  Scenario: Generate TypeScript function from spec
    Given a CLAUDE.md spec with function:
      | name          | signature                                    |
      | validateToken | validateToken(token: string): Promise<Claims> |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include:
      """
      export async function validateToken(token: string): Promise<Claims> {
      """

  Scenario: Generate TypeScript interface from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include:
      """
      export interface Claims {
        userId: string;
        role: Role;
        exp: number;
      }
      """

  Scenario: Generate TypeScript class from spec
    Given a CLAUDE.md spec with class:
      | name         | constructor_signature                          |
      | TokenManager | TokenManager(secret: string, algorithm?: string) |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include:
      """
      export class TokenManager {
        constructor(secret: string, algorithm?: string) {
      """

  Scenario: Generate TypeScript error class from behavior
    Given a CLAUDE.md spec with error behavior:
      | input         | output            |
      | expired token | TokenExpiredError |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include:
      """
      export class TokenExpiredError extends Error {
        constructor(message: string = 'Token expired') {
          super(message);
          this.name = 'TokenExpiredError';
        }
      }
      """

  # =============================================================================
  # Python Code Generation
  # =============================================================================

  Scenario: Generate Python async function from spec
    Given a CLAUDE.md spec with function:
      | name           | signature                                    |
      | validate_token | validate_token(token: str) -> Claims         |
    And the function is async
    And target language is "Python"
    When I generate code
    Then the generated code should include:
      """
      async def validate_token(token: str) -> Claims:
      """

  Scenario: Generate Python dataclass from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "Python"
    When I generate code
    Then the generated code should include:
      """
      @dataclass
      class Claims:
          user_id: str
          role: Role
          exp: int
      """

  Scenario: Generate Python exception from behavior
    Given a CLAUDE.md spec with error behavior:
      | input         | output            |
      | expired token | TokenExpiredError |
    And target language is "Python"
    When I generate code
    Then the generated code should include:
      """
      class TokenExpiredError(Exception):
          pass
      """

  # =============================================================================
  # Go Code Generation
  # =============================================================================

  Scenario: Generate Go function from spec
    Given a CLAUDE.md spec with function:
      | name          | signature                                   |
      | ValidateToken | ValidateToken(token string) (Claims, error) |
    And target language is "Go"
    When I generate code
    Then the generated code should include:
      """
      func ValidateToken(token string) (Claims, error) {
      """

  Scenario: Generate Go struct from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "Go"
    When I generate code
    Then the generated code should include:
      """
      type Claims struct {
      	UserID string `json:"userId"`
      	Role   Role   `json:"role"`
      	Exp    int64  `json:"exp"`
      }
      """

  Scenario: Generate Go error variable from behavior
    Given a CLAUDE.md spec with error behavior:
      | input         | output          |
      | expired token | ErrTokenExpired |
    And target language is "Go"
    When I generate code
    Then the generated code should include:
      """
      var ErrTokenExpired = errors.New("token expired")
      """

  # =============================================================================
  # Rust Code Generation
  # =============================================================================

  Scenario: Generate Rust async function from spec
    Given a CLAUDE.md spec with function:
      | name           | signature                                              |
      | validate_token | validate_token(token: &str) -> Result<Claims, Error>   |
    And the function is async
    And target language is "Rust"
    When I generate code
    Then the generated code should include:
      """
      pub async fn validate_token(token: &str) -> Result<Claims, Error> {
      """

  Scenario: Generate Rust struct from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "Rust"
    When I generate code
    Then the generated code should include:
      """
      #[derive(Debug, Clone, Serialize, Deserialize)]
      pub struct Claims {
          pub user_id: String,
          pub role: Role,
          pub exp: i64,
      }
      """

  Scenario: Generate Rust error enum from behavior
    Given a CLAUDE.md spec with error behaviors:
      | input           | output              |
      | expired token   | TokenError::Expired |
      | invalid token   | TokenError::Invalid |
    And target language is "Rust"
    When I generate code
    Then the generated code should include:
      """
      #[derive(Debug, Error)]
      pub enum TokenError {
          #[error("Token expired")]
          Expired,
          #[error("Invalid token")]
          Invalid,
      }
      """

  # =============================================================================
  # Java Code Generation
  # =============================================================================

  Scenario: Generate Java method from spec
    Given a CLAUDE.md spec with function:
      | name          | signature                                          |
      | validateToken | Claims validateToken(String token) throws AuthException |
    And target language is "Java"
    When I generate code
    Then the generated code should include:
      """
      public Claims validateToken(String token) throws AuthException {
      """

  Scenario: Generate Java record from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "Java"
    When I generate code
    Then the generated code should include:
      """
      public record Claims(
          String userId,
          Role role,
          long exp
      ) {}
      """

  Scenario: Generate Java exception from behavior
    Given a CLAUDE.md spec with error behavior:
      | input         | output                |
      | expired token | TokenExpiredException |
    And target language is "Java"
    When I generate code
    Then the generated code should include:
      """
      public class TokenExpiredException extends RuntimeException {
      """

  # =============================================================================
  # Kotlin Code Generation
  # =============================================================================

  Scenario: Generate Kotlin suspend function from spec
    Given a CLAUDE.md spec with function:
      | name          | signature                                |
      | validateToken | suspend fun validateToken(token: String): Claims |
    And target language is "Kotlin"
    When I generate code
    Then the generated code should include:
      """
      suspend fun validateToken(token: String): Claims {
      """

  Scenario: Generate Kotlin data class from spec
    Given a CLAUDE.md spec with type:
      | name   | definition                                    |
      | Claims | Claims { userId: string, role: Role, exp: number } |
    And target language is "Kotlin"
    When I generate code
    Then the generated code should include:
      """
      data class Claims(
          val userId: String,
          val role: Role,
          val exp: Long
      )
      """

  Scenario: Generate Kotlin exception from behavior
    Given a CLAUDE.md spec with error behavior:
      | input         | output                |
      | expired token | TokenExpiredException |
    And target language is "Kotlin"
    When I generate code
    Then the generated code should include:
      """
      class TokenExpiredException(message: String = "Token expired") : RuntimeException(message)
      """

  # =============================================================================
  # TDD Internal Workflow
  # =============================================================================

  Scenario: Generate test first then implementation
    Given a CLAUDE.md spec with behavior:
      | input           | output        |
      | valid JWT token | Claims object |
    And target language is "TypeScript"
    When I run the TDD workflow
    Then tests should be generated before implementation
    And tests should initially fail (RED phase)
    And implementation should make tests pass (GREEN phase)

  Scenario: Generate tests from behavior scenarios
    Given a CLAUDE.md spec with behaviors:
      | input           | output            | category |
      | valid JWT token | Claims object     | success  |
      | expired token   | TokenExpiredError | error    |
    And target language is "TypeScript"
    When I generate tests
    Then tests should include:
      """
      describe('validateToken', () => {
        it('should return Claims object for valid JWT token', async () => {
      """
    And tests should include:
      """
        it('should throw TokenExpiredError for expired token', async () => {
      """

  Scenario: Retry implementation when tests fail
    Given a generated implementation that fails tests
    When the TDD workflow detects failure
    Then the generator should retry up to 3 times
    And each retry should improve the implementation

  # =============================================================================
  # Conflict Handling
  # =============================================================================

  Scenario: Skip existing files by default
    Given an existing file "src/auth/token.ts" with content
    And a CLAUDE.md spec that would generate "src/auth/token.ts"
    When I run the generator with default options
    Then the existing file should NOT be overwritten
    And the generator should report skipped files

  Scenario: Overwrite existing files with --conflict overwrite
    Given an existing file "src/auth/token.ts" with content
    And a CLAUDE.md spec that would generate "src/auth/token.ts"
    When I run the generator with "--conflict overwrite"
    Then the existing file should be overwritten
    And the generator should report overwritten files

  # =============================================================================
  # Contract Implementation
  # =============================================================================

  Scenario: Generate validation from preconditions
    Given a CLAUDE.md spec with contract:
      | function      | preconditions                    |
      | validateToken | token must be non-empty string   |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include validation:
      """
      if (!token || token.length === 0) {
        throw new Error('token must be non-empty string');
      }
      """

  Scenario: Generate assertions from postconditions
    Given a CLAUDE.md spec with contract:
      | function      | postconditions                     |
      | validateToken | returns Claims with valid userId   |
    And target language is "TypeScript"
    When I generate tests
    Then the generated tests should include assertion:
      """
      expect(result.userId).toBeDefined();
      """

  # =============================================================================
  # Protocol Implementation
  # =============================================================================

  Scenario: Generate state enum from protocol
    Given a CLAUDE.md spec with protocol states:
      | state        |
      | Disconnected |
      | Connecting   |
      | Connected    |
      | Error        |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include:
      """
      export enum ConnectionState {
        Disconnected = 'Disconnected',
        Connecting = 'Connecting',
        Connected = 'Connected',
        Error = 'Error',
      }
      """

  Scenario: Generate lifecycle methods from protocol
    Given a CLAUDE.md spec with lifecycle:
      | order | method  |
      | 1     | init    |
      | 2     | start   |
      | 3     | stop    |
      | 4     | destroy |
    And target language is "TypeScript"
    When I generate code
    Then the generated code should include methods in order:
      | method  |
      | init    |
      | start   |
      | stop    |
      | destroy |

  # =============================================================================
  # Complete Generation Flow
  # =============================================================================

  Scenario: Generate complete module from CLAUDE.md
    Given a CLAUDE.md file at "fixtures/generate/auth/CLAUDE.md" with content:
      """
      # auth

      ## Purpose
      User authentication module.

      ## Exports

      ### Functions
      - `validateToken(token: string): Promise<Claims>`

      ### Types
      - `Claims { userId: string, role: Role }`

      ## Behavior

      ### Success Cases
      - valid JWT token → Claims object

      ### Error Cases
      - expired token → TokenExpiredError
      """
    And target language is "TypeScript"
    When I run the complete generate workflow
    Then the following files should be generated:
      | file         |
      | index.ts     |
      | types.ts     |
      | errors.ts    |
      | index.test.ts |
    And all generated tests should pass

  # =============================================================================
  # Output Report
  # =============================================================================

  Scenario: Generate summary report after completion
    Given a CLAUDE.md spec that generates multiple files
    When I run the generator
    Then the output should include a summary:
      | field           |
      | generated_files |
      | skipped_files   |
      | tests_passed    |
      | tests_failed    |
