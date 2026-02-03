Feature: CLAUDE.md Parser
  As a generator agent
  I want to parse CLAUDE.md files into structured specs
  So that I can generate source code from the specification

  Background:
    Given the claude-md-parser uses regex patterns for section parsing
    And the parser produces JSON output compatible with code generation

  # =============================================================================
  # Basic Parsing
  # =============================================================================

  Scenario: Parse Purpose section
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Handles user authentication and token validation.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims object
      """
    When I parse the CLAUDE.md file
    Then the spec should have purpose "Handles user authentication and token validation."

  Scenario: Parse Exports section with functions
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports

      ### Functions
      - `validateToken(token: string): Promise<Claims>`
      - `generateToken(userId: string, role: Role): string`

      ## Behavior
      - valid token → Claims
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name          | signature                                    |
      | validateToken | validateToken(token: string): Promise<Claims> |
      | generateToken | generateToken(userId: string, role: Role): string |

  Scenario: Parse Exports section with types
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports

      ### Types
      - `Claims { userId: string, role: Role, exp: number }`
      - `TokenConfig { secret: string, expiresIn: number }`

      ### Functions
      - `validate(): void`

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported types:
      | name        | definition                                    |
      | Claims      | Claims { userId: string, role: Role, exp: number } |
      | TokenConfig | TokenConfig { secret: string, expiresIn: number } |

  Scenario: Parse Exports section with classes
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports

      ### Classes
      - `TokenManager(secret: string, algorithm?: string)`
      - `AuthService(config: AuthConfig)`

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported classes:
      | name         | constructor_signature                          |
      | TokenManager | TokenManager(secret: string, algorithm?: string) |
      | AuthService  | AuthService(config: AuthConfig)                |

  # =============================================================================
  # Dependencies Parsing
  # =============================================================================

  Scenario: Parse Dependencies section
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Dependencies
      - external: jsonwebtoken@9.0.0
      - external: bcrypt@5.0.0
      - internal: ../utils/crypto
      - internal: ./types

      ## Exports
      - `validate(): void`

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then the spec should have external dependencies:
      | package           |
      | jsonwebtoken@9.0.0 |
      | bcrypt@5.0.0      |
    And the spec should have internal dependencies:
      | path           |
      | ../utils/crypto |
      | ./types        |

  # =============================================================================
  # Behavior Parsing
  # =============================================================================

  Scenario: Parse Behavior section with scenarios
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior

      ### Success Cases
      - valid JWT token → Claims object with userId and role
      - valid token with admin role → Claims with admin permissions

      ### Error Cases
      - expired token → TokenExpiredError
      - invalid signature → InvalidTokenError
      - malformed token → MalformedTokenError
      """
    When I parse the CLAUDE.md file
    Then the spec should have behaviors:
      | input                    | output                           | category |
      | valid JWT token          | Claims object with userId and role | success  |
      | valid token with admin role | Claims with admin permissions   | success  |
      | expired token            | TokenExpiredError                | error    |
      | invalid signature        | InvalidTokenError                | error    |
      | malformed token          | MalformedTokenError              | error    |

  # =============================================================================
  # Contract Parsing
  # =============================================================================

  Scenario: Parse Contract section
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports
      - `validateToken(token: string): Promise<Claims>`

      ## Behavior
      - valid token → Claims

      ## Contract

      ### validateToken
      - **Preconditions**: token must be non-empty string
      - **Postconditions**: returns Claims with valid userId
      - **Throws**: InvalidTokenError if token is malformed
      """
    When I parse the CLAUDE.md file
    Then the spec should have contract for "validateToken":
      | preconditions               | postconditions                  | throws            |
      | token must be non-empty string | returns Claims with valid userId | InvalidTokenError |

  Scenario: Parse Contract with multiple functions
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Exports
      - `validateToken(token: string): Claims`
      - `generateToken(userId: string): string`

      ## Behavior
      - valid token → Claims
      - valid userId → token

      ## Contract

      ### validateToken
      - **Preconditions**: token must be JWT format
      - **Postconditions**: Claims.exp > current time
      - **Throws**: TokenExpiredError, InvalidTokenError

      ### generateToken
      - **Preconditions**: userId is valid UUID
      - **Postconditions**: returns signed JWT token
      - **Invariants**: token contains userId
      """
    When I parse the CLAUDE.md file
    Then the spec should have contract for "validateToken":
      | preconditions           | postconditions           | throws                           |
      | token must be JWT format | Claims.exp > current time | TokenExpiredError, InvalidTokenError |
    And the spec should have contract for "generateToken":
      | preconditions       | postconditions          | invariants           |
      | userId is valid UUID | returns signed JWT token | token contains userId |

  Scenario: Parse Contract with multiple preconditions
    Given a CLAUDE.md file with content:
      """
      # user-module

      ## Purpose
      User management.

      ## Exports
      - `createUser(name: string, email: string): User`

      ## Behavior
      - valid input → User

      ## Contract

      ### createUser
      - **Preconditions**: name is non-empty
      - **Preconditions**: email is valid format
      - **Preconditions**: email is unique in database
      - **Postconditions**: User.id is assigned
      - **Postconditions**: User.createdAt is set
      """
    When I parse the CLAUDE.md file
    Then the spec should have contract for "createUser" with 3 preconditions
    And the spec should have contract for "createUser" with 2 postconditions

  # =============================================================================
  # Protocol Parsing
  # =============================================================================

  Scenario: Parse Protocol section with state machine
    Given a CLAUDE.md file with content:
      """
      # connection-manager

      ## Purpose
      Manages WebSocket connections.

      ## Exports
      - `connect(): void`

      ## Behavior
      - connect → Connected state

      ## Protocol

      ### State Machine
      States: `Disconnected` | `Connecting` | `Connected` | `Error`

      Transitions:
      - `Disconnected` + `connect()` → `Connecting`
      - `Connecting` + `success` → `Connected`
      - `Connecting` + `failure` → `Error`
      - `Connected` + `disconnect()` → `Disconnected`
      """
    When I parse the CLAUDE.md file
    Then the spec should have protocol with states:
      | state        |
      | Disconnected |
      | Connecting   |
      | Connected    |
      | Error        |
    And the spec should have protocol transitions:
      | from         | trigger      | to           |
      | Disconnected | connect()    | Connecting   |
      | Connecting   | success      | Connected    |
      | Connecting   | failure      | Error        |
      | Connected    | disconnect() | Disconnected |

  Scenario: Parse Protocol section with lifecycle
    Given a CLAUDE.md file with content:
      """
      # service-manager

      ## Purpose
      Manages service lifecycle.

      ## Exports
      - `init(): void`

      ## Behavior
      - init → initialized state

      ## Protocol

      ### Lifecycle
      1. `init()` - Initialize resources
      2. `start()` - Start processing
      3. `stop()` - Stop processing
      4. `destroy()` - Clean up resources
      """
    When I parse the CLAUDE.md file
    Then the spec should have protocol lifecycle:
      | order | method    | description         |
      | 1     | init      | Initialize resources |
      | 2     | start     | Start processing     |
      | 3     | stop      | Stop processing      |
      | 4     | destroy   | Clean up resources   |

  Scenario: Parse Protocol with both state machine and lifecycle
    Given a CLAUDE.md file with content:
      """
      # worker-manager

      ## Purpose
      Manages background workers.

      ## Exports
      - `start(): void`
      - `stop(): void`

      ## Behavior
      - start → Running state
      - stop → Stopped state

      ## Protocol

      ### State Machine
      States: `Idle` | `Starting` | `Running` | `Stopping` | `Stopped` | `Error`

      Transitions:
      - `Idle` + `start()` → `Starting`
      - `Starting` + `ready` → `Running`
      - `Starting` + `error` → `Error`
      - `Running` + `stop()` → `Stopping`
      - `Stopping` + `done` → `Stopped`
      - `Error` + `reset()` → `Idle`

      ### Lifecycle
      1. `initialize()` - Load configuration
      2. `start()` - Start worker threads
      3. `healthCheck()` - Verify worker health
      4. `stop()` - Graceful shutdown
      5. `cleanup()` - Release resources
      """
    When I parse the CLAUDE.md file
    Then the spec should have protocol with states:
      | state    |
      | Idle     |
      | Starting |
      | Running  |
      | Stopping |
      | Stopped  |
      | Error    |
    And the spec should have protocol transitions:
      | from     | trigger  | to       |
      | Idle     | start()  | Starting |
      | Starting | ready    | Running  |
      | Starting | error    | Error    |
      | Running  | stop()   | Stopping |
      | Stopping | done     | Stopped  |
      | Error    | reset()  | Idle     |
    And the spec should have protocol lifecycle:
      | order | method       | description           |
      | 1     | initialize   | Load configuration    |
      | 2     | start        | Start worker threads  |
      | 3     | healthCheck  | Verify worker health  |
      | 4     | stop         | Graceful shutdown     |
      | 5     | cleanup      | Release resources     |

  Scenario: Parse Protocol with conditional transitions
    Given a CLAUDE.md file with content:
      """
      # payment-processor

      ## Purpose
      Processes payments.

      ## Exports
      - `process(payment: Payment): Result`

      ## Behavior
      - valid payment → success
      - invalid payment → error

      ## Protocol

      ### State Machine
      States: `Pending` | `Validating` | `Processing` | `Completed` | `Failed` | `Refunded`

      Transitions:
      - `Pending` + `submit()` → `Validating`
      - `Validating` + `valid` → `Processing`
      - `Validating` + `invalid` → `Failed`
      - `Processing` + `success` → `Completed`
      - `Processing` + `declined` → `Failed`
      - `Completed` + `refund()` → `Refunded`
      """
    When I parse the CLAUDE.md file
    Then the spec should have protocol with 6 states
    And the spec should have protocol with 6 transitions

  # =============================================================================
  # Structure Parsing
  # =============================================================================

  Scenario: Parse Structure section
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Structure
      - jwt/: JWT token handling (see jwt/CLAUDE.md)
      - session.ts: Session management logic
      - types.ts: Authentication types
      - index.ts: Module entry point

      ## Exports
      - `validate(): void`

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then the spec should have structure with subdirs:
      | name | description                          |
      | jwt  | JWT token handling (see jwt/CLAUDE.md) |
    And the spec should have structure with files:
      | name       | description              |
      | session.ts | Session management logic |
      | types.ts   | Authentication types     |
      | index.ts   | Module entry point       |

  # =============================================================================
  # Multi-Language Support
  # =============================================================================

  Scenario: Parse Python-style exports
    Given a CLAUDE.md file with content:
      """
      # auth_module

      ## Purpose
      Python authentication module.

      ## Exports

      ### Functions
      - `validate_token(token: str) -> Claims`
      - `generate_token(user_id: str, role: Role) -> str`

      ### Classes
      - `TokenManager(secret: str, algorithm: str = "HS256")`

      ## Behavior
      - valid token → Claims
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name           | signature                                  |
      | validate_token | validate_token(token: str) -> Claims       |
      | generate_token | generate_token(user_id: str, role: Role) -> str |

  Scenario: Parse Go-style exports
    Given a CLAUDE.md file with content:
      """
      # auth

      ## Purpose
      Go authentication package.

      ## Exports

      ### Functions
      - `ValidateToken(token string) (Claims, error)`
      - `GenerateToken(userID string, role Role) (string, error)`

      ### Types
      - `Claims struct { UserID string; Role Role; Exp int64 }`

      ## Behavior
      - valid token → Claims, nil
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name          | signature                                     |
      | ValidateToken | ValidateToken(token string) (Claims, error)   |
      | GenerateToken | GenerateToken(userID string, role Role) (string, error) |

  Scenario: Parse Rust-style exports
    Given a CLAUDE.md file with content:
      """
      # auth

      ## Purpose
      Rust authentication module.

      ## Exports

      ### Functions
      - `validate_token(token: &str) -> Result<Claims, AuthError>`
      - `generate_token(user_id: &str, role: Role) -> Result<String, AuthError>`

      ### Structs
      - `Claims { user_id: String, role: Role, exp: i64 }`

      ## Behavior
      - valid token → Ok(Claims)
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name           | signature                                              |
      | validate_token | validate_token(token: &str) -> Result<Claims, AuthError> |
      | generate_token | generate_token(user_id: &str, role: Role) -> Result<String, AuthError> |

  Scenario: Parse Java-style exports
    Given a CLAUDE.md file with content:
      """
      # AuthService

      ## Purpose
      Java authentication service.

      ## Exports

      ### Methods
      - `Claims validateToken(String token) throws AuthException`
      - `String generateToken(String userId, Role role)`

      ### Classes
      - `TokenManager(String secret, Algorithm algorithm)`

      ## Behavior
      - valid token → Claims object
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name          | signature                                          |
      | validateToken | Claims validateToken(String token) throws AuthException |
      | generateToken | String generateToken(String userId, Role role)     |

  Scenario: Parse Kotlin-style exports
    Given a CLAUDE.md file with content:
      """
      # AuthService

      ## Purpose
      Kotlin authentication service.

      ## Exports

      ### Functions
      - `suspend fun validateToken(token: String): Claims`
      - `fun generateToken(userId: String, role: Role): String`

      ### Data Classes
      - `data class Claims(val userId: String, val role: Role, val exp: Long)`

      ## Behavior
      - valid token → Claims object
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name          | signature                                      |
      | validateToken | suspend fun validateToken(token: String): Claims |
      | generateToken | fun generateToken(userId: String, role: Role): String |

  # =============================================================================
  # Generic Type Parsing (P0)
  # =============================================================================

  Scenario: Parse function with nested generic types
    Given a CLAUDE.md file with content:
      """
      # cache-module

      ## Purpose
      Cache management module.

      ## Exports

      ### Functions
      - `getCache(key: string): Map<string, List<CacheEntry>>`
      - `setCache(key: string, value: Map<string, any>): void`

      ## Behavior
      - valid key → cached value
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name     | signature                                              |
      | getCache | getCache(key: string): Map<string, List<CacheEntry>>   |
      | setCache | setCache(key: string, value: Map<string, any>): void   |

  Scenario: Parse function with complex generic parameters
    Given a CLAUDE.md file with content:
      """
      # transform-module

      ## Purpose
      Data transformation module.

      ## Exports

      ### Functions
      - `transform<T, U>(input: T, mapper: (item: T) => U): U`
      - `batchProcess(items: Array<Pair<string, number>>): Result<void, ProcessError>`

      ## Behavior
      - input → transformed output
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name         | signature                                                              |
      | transform    | transform<T, U>(input: T, mapper: (item: T) => U): U                   |
      | batchProcess | batchProcess(items: Array<Pair<string, number>>): Result<void, ProcessError> |

  Scenario: Parse type with generic fields
    Given a CLAUDE.md file with content:
      """
      # data-module

      ## Purpose
      Data structures module.

      ## Exports

      ### Types
      - `PagedResult<T> { items: Array<T>, total: number, page: number }`
      - `Either<L, R> { left: L | null, right: R | null }`

      ### Functions
      - `getPage(): PagedResult<User>`

      ## Behavior
      - request → paged result
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported types:
      | name        | definition                                            |
      | PagedResult | PagedResult<T> { items: Array<T>, total: number, page: number } |
      | Either      | Either<L, R> { left: L | null, right: R | null }      |

  Scenario: Parse deeply nested generic types
    Given a CLAUDE.md file with content:
      """
      # nested-module

      ## Purpose
      Nested types module.

      ## Exports

      ### Functions
      - `getData(): Result<Option<Map<string, List<Pair<K, V>>>>, Error>`

      ## Behavior
      - request → nested data
      """
    When I parse the CLAUDE.md file
    Then the spec should have exported functions:
      | name    | signature                                                        |
      | getData | getData(): Result<Option<Map<string, List<Pair<K, V>>>>, Error>  |

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: Handle empty Exports section
    Given a CLAUDE.md file with content:
      """
      # constants

      ## Purpose
      Application constants.

      ## Exports
      None

      ## Behavior
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have no exports

  Scenario: Handle missing optional sections
    Given a CLAUDE.md file with content:
      """
      # simple-module

      ## Purpose
      A simple module.

      ## Exports
      - `doSomething(): void`

      ## Behavior
      - call → result
      """
    When I parse the CLAUDE.md file
    Then the spec should not have dependencies
    And the spec should not have contracts
    And the spec should not have protocol

  Scenario: Fail fast on missing Purpose section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Exports
      - `validate(): void`

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Purpose"

  Scenario: Fail fast on missing Exports section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Purpose
      Some purpose description.

      ## Behavior
      - input → output
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Exports"

  Scenario: Fail fast on missing Behavior section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Purpose
      Some purpose description.

      ## Exports
      - `validate(): void`
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Behavior"

  Scenario: Fail fast on completely malformed CLAUDE.md
    Given a CLAUDE.md file with content:
      """
      This is not a valid CLAUDE.md file.
      It has no proper sections.
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Purpose"

  # =============================================================================
  # CLI Integration
  # =============================================================================

  Scenario: CLI parse-claude-md command outputs JSON
    Given a CLAUDE.md file at "fixtures/generate/sample/CLAUDE.md"
    When I run "claude-md-core parse-claude-md --file fixtures/generate/sample/CLAUDE.md"
    Then the output should be valid JSON
    And the JSON should have "purpose" field
    And the JSON should have "exports" object
    And the JSON should have "behaviors" array
