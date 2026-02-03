Feature: Signature Converter
  As a generator agent
  I want to convert language-agnostic signatures to target language syntax
  So that I can generate code in the user's preferred language

  Background:
    Given the signature converter supports 6 target languages
    And the converter preserves semantic meaning across conversions

  # =============================================================================
  # TypeScript Conversion
  # =============================================================================

  Scenario: Convert function signature to TypeScript
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to TypeScript
    Then the result should be "async function validateToken(token: string): Promise<Claims>"

  Scenario: Convert sync function to TypeScript
    Given a signature "hashPassword(password: string): string"
    When I convert to TypeScript
    Then the result should be "function hashPassword(password: string): string"

  Scenario: Convert function with optional params to TypeScript
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to TypeScript
    Then the result should be "function createUser(name: string, options?: CreateOptions): User"

  Scenario: Convert type definition to TypeScript
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to TypeScript
    Then the result should be:
      """
      interface Claims {
        userId: string;
        role: Role;
        exp: number;
      }
      """

  Scenario: Convert class to TypeScript
    Given a class "TokenManager(secret: string, algorithm?: string)"
    When I convert to TypeScript
    Then the result should be:
      """
      class TokenManager {
        constructor(secret: string, algorithm?: string) {}
      }
      """

  # =============================================================================
  # Python Conversion
  # =============================================================================

  Scenario: Convert async function signature to Python
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to Python
    Then the result should be "async def validate_token(token: str) -> Claims:"

  Scenario: Convert sync function to Python
    Given a signature "hashPassword(password: string): string"
    When I convert to Python
    Then the result should be "def hash_password(password: str) -> str:"

  Scenario: Convert function with optional params to Python
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to Python
    Then the result should be "def create_user(name: str, options: CreateOptions | None = None) -> User:"

  Scenario: Convert type definition to Python dataclass
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to Python
    Then the result should be:
      """
      @dataclass
      class Claims:
          user_id: str
          role: Role
          exp: int
      """

  Scenario: Convert class to Python
    Given a class "TokenManager(secret: string, algorithm?: string)"
    When I convert to Python
    Then the result should be:
      """
      class TokenManager:
          def __init__(self, secret: str, algorithm: str | None = None):
              pass
      """

  # =============================================================================
  # Go Conversion
  # =============================================================================

  Scenario: Convert function signature to Go
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to Go
    Then the result should be "func ValidateToken(token string) (Claims, error)"

  Scenario: Convert sync function to Go
    Given a signature "hashPassword(password: string): string"
    When I convert to Go
    Then the result should be "func HashPassword(password string) string"

  Scenario: Convert function with optional params to Go (variadic)
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to Go
    Then the result should be "func CreateUser(name string, opts ...CreateOptions) (User, error)"

  Scenario: Convert type definition to Go struct
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to Go
    Then the result should be:
      """
      type Claims struct {
      	UserID string `json:"userId"`
      	Role   Role   `json:"role"`
      	Exp    int64  `json:"exp"`
      }
      """

  Scenario: Convert error type to Go
    Given an error type "TokenExpiredError"
    When I convert to Go
    Then the result should be "var ErrTokenExpired = errors.New(\"token expired\")"

  # =============================================================================
  # Rust Conversion
  # =============================================================================

  Scenario: Convert async function signature to Rust
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to Rust
    Then the result should be "pub async fn validate_token(token: &str) -> Result<Claims, Error>"

  Scenario: Convert sync function to Rust
    Given a signature "hashPassword(password: string): string"
    When I convert to Rust
    Then the result should be "pub fn hash_password(password: &str) -> String"

  Scenario: Convert function with optional params to Rust
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to Rust
    Then the result should be "pub fn create_user(name: &str, options: Option<CreateOptions>) -> Result<User, Error>"

  Scenario: Convert type definition to Rust struct
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to Rust
    Then the result should be:
      """
      #[derive(Debug, Clone, Serialize, Deserialize)]
      pub struct Claims {
          pub user_id: String,
          pub role: Role,
          pub exp: i64,
      }
      """

  Scenario: Convert error type to Rust enum variant
    Given an error type "TokenExpiredError"
    When I convert to Rust
    Then the result should include "#[error(\"Token expired\")]"
    And the result should include "TokenExpired"

  # =============================================================================
  # Java Conversion
  # =============================================================================

  Scenario: Convert async function signature to Java
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to Java
    Then the result should be "public CompletableFuture<Claims> validateToken(String token)"

  Scenario: Convert sync function to Java
    Given a signature "hashPassword(password: string): string"
    When I convert to Java
    Then the result should be "public String hashPassword(String password)"

  Scenario: Convert function with optional params to Java
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to Java
    Then the result should be "public User createUser(String name, CreateOptions options)"

  Scenario: Convert type definition to Java record
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to Java
    Then the result should be:
      """
      public record Claims(
          String userId,
          Role role,
          long exp
      ) {}
      """

  Scenario: Convert error type to Java exception
    Given an error type "TokenExpiredError"
    When I convert to Java
    Then the result should be:
      """
      public class TokenExpiredException extends RuntimeException {
          public TokenExpiredException(String message) {
              super(message);
          }
      }
      """

  # =============================================================================
  # Kotlin Conversion
  # =============================================================================

  Scenario: Convert async function signature to Kotlin
    Given a signature "validateToken(token: string): Promise<Claims>"
    When I convert to Kotlin
    Then the result should be "suspend fun validateToken(token: String): Claims"

  Scenario: Convert sync function to Kotlin
    Given a signature "hashPassword(password: string): string"
    When I convert to Kotlin
    Then the result should be "fun hashPassword(password: String): String"

  Scenario: Convert function with optional params to Kotlin
    Given a signature "createUser(name: string, options?: CreateOptions): User"
    When I convert to Kotlin
    Then the result should be "fun createUser(name: String, options: CreateOptions? = null): User"

  Scenario: Convert type definition to Kotlin data class
    Given a type "Claims { userId: string, role: Role, exp: number }"
    When I convert to Kotlin
    Then the result should be:
      """
      data class Claims(
          val userId: String,
          val role: Role,
          val exp: Long
      )
      """

  Scenario: Convert error type to Kotlin exception
    Given an error type "TokenExpiredError"
    When I convert to Kotlin
    Then the result should be "class TokenExpiredException(message: String) : RuntimeException(message)"

  # =============================================================================
  # Type Mapping
  # =============================================================================

  Scenario Outline: Map primitive types correctly
    Given a signature with type "<source_type>"
    When I convert to <target_lang>
    Then the type should be mapped to "<target_type>"

    Examples:
      | source_type | target_lang | target_type       |
      | string      | TypeScript  | string            |
      | string      | Python      | str               |
      | string      | Go          | string            |
      | string      | Rust        | String            |
      | string      | Java        | String            |
      | string      | Kotlin      | String            |
      | number      | TypeScript  | number            |
      | number      | Python      | int               |
      | number      | Go          | int64             |
      | number      | Rust        | i64               |
      | number      | Java        | long              |
      | number      | Kotlin      | Long              |
      | boolean     | TypeScript  | boolean           |
      | boolean     | Python      | bool              |
      | boolean     | Go          | bool              |
      | boolean     | Rust        | bool              |
      | boolean     | Java        | boolean           |
      | boolean     | Kotlin      | Boolean           |
      | void        | TypeScript  | void              |
      | void        | Python      | None              |
      | void        | Go          |                   |
      | void        | Rust        | ()                |
      | void        | Java        | void              |
      | void        | Kotlin      | Unit              |

  # =============================================================================
  # Naming Convention Conversion
  # =============================================================================

  Scenario Outline: Convert naming conventions
    Given a function name "<original_name>"
    When I convert to <target_lang>
    Then the function name should be "<converted_name>"

    Examples:
      | original_name   | target_lang | converted_name    |
      | validateToken   | TypeScript  | validateToken     |
      | validateToken   | Python      | validate_token    |
      | validateToken   | Go          | ValidateToken     |
      | validateToken   | Rust        | validate_token    |
      | validateToken   | Java        | validateToken     |
      | validateToken   | Kotlin      | validateToken     |
      | validate_token  | TypeScript  | validateToken     |
      | validate_token  | Python      | validate_token    |
      | validate_token  | Go          | ValidateToken     |
      | validate_token  | Rust        | validate_token    |
      | validate_token  | Java        | validateToken     |
      | validate_token  | Kotlin      | validateToken     |

  # =============================================================================
  # Complex Type Conversion
  # =============================================================================

  Scenario: Convert array type
    Given a signature "getUsers(): User[]"
    When I convert to Go
    Then the result should be "func GetUsers() []User"

  Scenario: Convert map type
    Given a signature "getConfig(): Map<string, any>"
    When I convert to Rust
    Then the result should be "pub fn get_config() -> HashMap<String, Value>"

  Scenario: Convert generic type
    Given a signature "findById<T>(id: string): T"
    When I convert to Java
    Then the result should be "public <T> T findById(String id)"

  # =============================================================================
  # Nested Generic Type Conversion (P0)
  # =============================================================================

  Scenario: Convert nested generic parameters correctly
    Given a signature "getCache(key: string): Map<string, List<CacheEntry>>"
    When I convert to TypeScript
    Then the result should be "function getCache(key: string): Map<string, List<CacheEntry>>"

  Scenario: Convert nested generic parameters to Python
    Given a signature "getCache(key: string): Map<string, List<CacheEntry>>"
    When I convert to Python
    Then the result should be "def get_cache(key: str) -> Dict[str, List[CacheEntry]]:"

  Scenario: Convert nested generic parameters to Go
    Given a signature "transform(data: Map<string, List<Item>>): Result<void, Error>"
    When I convert to Go
    Then the result should be "func Transform(data map[string][]Item) error"

  Scenario: Convert nested generic parameters to Rust
    Given a signature "process(input: Array<Pair<K, V>>): Map<K, V>"
    When I convert to Rust
    Then the result should be "pub fn process(input: Vec<(K, V)>) -> HashMap<K, V>"

  Scenario: Convert generic function with multiple type parameters
    Given a signature "transform<T, U, V>(input: T, mapper: Fn<T, U>, filter: Fn<U, V>): V"
    When I convert to TypeScript
    Then the result should be "function transform<T, U, V>(input: T, mapper: Fn<T, U>, filter: Fn<U, V>): V"

  Scenario: Convert deeply nested generic to Java
    Given a signature "getData(): Result<Option<Map<string, List<Pair<K, V>>>>, Error>"
    When I convert to Java
    Then the result should be "public Result<Optional<Map<String, List<Pair<K, V>>>>, Error> getData()"

  Scenario: Parse parameters with comma inside generic type
    Given a signature "merge(a: Map<string, number>, b: Map<string, number>): Map<string, number>"
    When I convert to Python
    Then the result should be "def merge(a: Dict[str, int], b: Dict[str, int]) -> Dict[str, int]:"

  # =============================================================================
  # Error Handling Patterns
  # =============================================================================

  Scenario: Convert throwing function to Result-based (Rust)
    Given a signature "validateToken(token: string): Claims throws InvalidTokenError"
    When I convert to Rust
    Then the result should be "pub fn validate_token(token: &str) -> Result<Claims, InvalidTokenError>"

  Scenario: Convert throwing function to error tuple (Go)
    Given a signature "validateToken(token: string): Claims throws InvalidTokenError"
    When I convert to Go
    Then the result should be "func ValidateToken(token string) (Claims, error)"

  Scenario: Convert throwing function to throws declaration (Java)
    Given a signature "validateToken(token: string): Claims throws InvalidTokenError"
    When I convert to Java
    Then the result should be "public Claims validateToken(String token) throws InvalidTokenException"

  # =============================================================================
  # CLI Integration
  # =============================================================================

  Scenario: CLI convert-signature command
    When I run "claude-md-core convert-signature --signature 'validateToken(token: string): Promise<Claims>' --target-lang go"
    Then the output should contain "func ValidateToken(token string) (Claims, error)"

  Scenario: CLI convert-signature with JSON output
    When I run "claude-md-core convert-signature --signature 'validateToken(token: string): Claims' --target-lang rust --output result.json"
    Then "result.json" should contain valid JSON
    And the JSON should have "converted_signature" field
    And the JSON should have "target_language" field with value "rust"
