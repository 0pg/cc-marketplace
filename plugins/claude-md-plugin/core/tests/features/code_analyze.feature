Feature: Code Analyze Skill
  As an extractor agent
  I want to analyze source code and extract structured information
  So that I can generate accurate CLAUDE.md documentation

  Background:
    Given the code-analyze skill uses only Read, Glob, and Grep tools
    And regex patterns are used for language-specific analysis

  # =============================================================================
  # TypeScript Analysis
  # =============================================================================

  Scenario: Extract TypeScript function exports
    Given a TypeScript file "fixtures/typescript/index.ts"
    When I analyze the file for exports
    Then I should find exported functions:
      | name          | signature                                      |
      | validateToken | validateToken(token: string): Promise<Claims>  |
      | generateToken | generateToken(userId: string, role: string): string |

  Scenario: Extract TypeScript type exports
    Given a TypeScript file "fixtures/typescript/types.ts"
    When I analyze the file for exports
    Then I should find exported types:
      | name         | kind      |
      | Claims       | interface |
      | Role         | type      |
      | TokenConfig  | interface |
      | TokenPayload | type      |

  Scenario: Extract TypeScript class exports
    Given a TypeScript file "fixtures/typescript/index.ts"
    When I analyze the file for exports
    Then I should find exported classes:
      | name              |
      | TokenExpiredError |
      | InvalidTokenError |

  Scenario: Extract TypeScript dependencies
    Given a TypeScript file "fixtures/typescript/index.ts"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | package       |
      | jsonwebtoken  |
    And I should find internal dependencies:
      | path            |
      | ./types         |
      | ../utils/crypto |

  # =============================================================================
  # Python Analysis
  # =============================================================================

  Scenario: Extract Python exports via __all__
    Given a Python package "fixtures/python"
    When I analyze the package for exports
    Then I should find symbols defined in __all__:
      | name           | kind     |
      | validate_token | function |
      | generate_token | function |
      | TokenManager   | class    |
      | AuthResult     | class    |

  Scenario: Extract Python class exports
    Given a Python file "fixtures/python/auth.py"
    When I analyze the file for exports
    Then I should find exported classes:
      | name         |
      | AuthResult   |
      | TokenManager |
    And I should NOT find private functions:
      | name             |
      | _internal_helper |

  Scenario: Extract Python dependencies
    Given a Python file "fixtures/python/auth.py"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | package |
      | jwt     |
    And I should find internal dependencies:
      | path   |
      | .types |

  # =============================================================================
  # Go Analysis
  # =============================================================================

  Scenario: Extract Go exports by capitalization
    Given a Go file "fixtures/go/token.go"
    When I analyze the file for exports
    Then I should find exported functions (capitalized):
      | name          |
      | ValidateToken |
      | GenerateToken |
    And I should NOT find private functions:
      | name           |
      | internalHelper |

  Scenario: Extract Go type exports
    Given a Go file "fixtures/go/token.go"
    When I analyze the file for exports
    Then I should find exported types (capitalized):
      | name   | kind   |
      | Claims | struct |
      | Config | struct |

  Scenario: Extract Go error variables
    Given a Go file "fixtures/go/token.go"
    When I analyze the file for exports
    Then I should find exported error variables:
      | name            |
      | ErrExpiredToken |
      | ErrInvalidToken |

  Scenario: Extract Go dependencies
    Given a Go file "fixtures/go/token.go"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | package                      |
      | github.com/golang-jwt/jwt/v5 |

  # =============================================================================
  # Rust Analysis
  # =============================================================================

  Scenario: Extract Rust exports by pub keyword
    Given a Rust file "fixtures/rust/lib.rs"
    When I analyze the file for exports
    Then I should find pub functions:
      | name           |
      | validate_token |
      | generate_token |
    And I should NOT find private functions:
      | name            |
      | internal_helper |

  Scenario: Extract Rust type exports
    Given a Rust file "fixtures/rust/lib.rs"
    When I analyze the file for exports
    Then I should find pub types:
      | name        | kind   |
      | Claims      | struct |
      | Role        | enum   |
      | TokenConfig | struct |
      | TokenError  | enum   |

  Scenario: Extract Rust dependencies
    Given a Rust file "fixtures/rust/lib.rs"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | crate        |
      | jsonwebtoken |
      | serde        |
      | thiserror    |

  # =============================================================================
  # Java Analysis
  # =============================================================================

  Scenario: Extract Java exports by public keyword
    Given a Java file "fixtures/java/TokenService.java"
    When I analyze the file for exports
    Then I should find public methods:
      | name          |
      | validateToken |
      | generateToken |
    And I should NOT find private methods:
      | name             |
      | mapToTokenClaims |
      | isTokenExpired   |

  Scenario: Extract Java class exports
    Given a Java directory "fixtures/java"
    When I analyze the directory for exports
    Then I should find public classes:
      | name                  |
      | TokenService          |
      | TokenClaims           |
      | TokenConfig           |
      | TokenExpiredException |
      | InvalidTokenException |

  Scenario: Extract Java enum exports
    Given a Java file "fixtures/java/Role.java"
    When I analyze the file for exports
    Then I should find public enums:
      | name |
      | Role |

  Scenario: Extract Java dependencies
    Given a Java file "fixtures/java/TokenService.java"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | package        |
      | io.jsonwebtoken |

  Scenario: Infer Java error behavior from throws
    Given a Java file "fixtures/java/TokenService.java"
    When I analyze the file for behaviors
    Then I should infer error behaviors:
      | input         | output                |
      | Expired token | TokenExpiredException |
      | Invalid token | InvalidTokenException |

  # =============================================================================
  # Kotlin Analysis
  # =============================================================================

  Scenario: Extract Kotlin exports (default public)
    Given a Kotlin file "fixtures/kotlin/TokenService.kt"
    When I analyze the file for exports
    Then I should find public functions:
      | name          |
      | validateToken |
      | generateToken |
    And I should NOT find private functions:
      | name             |
      | mapToTokenClaims |
      | isTokenExpired   |

  Scenario: Extract Kotlin data class exports
    Given a Kotlin directory "fixtures/kotlin"
    When I analyze the directory for exports
    Then I should find data classes:
      | name        |
      | TokenClaims |
      | TokenConfig |

  Scenario: Extract Kotlin enum class exports
    Given a Kotlin file "fixtures/kotlin/Role.kt"
    When I analyze the file for exports
    Then I should find enum classes:
      | name |
      | Role |

  Scenario: Extract Kotlin dependencies
    Given a Kotlin file "fixtures/kotlin/TokenService.kt"
    When I analyze the file for dependencies
    Then I should find external dependencies:
      | package        |
      | io.jsonwebtoken |

  Scenario: Infer Kotlin error behavior from Result type
    Given a Kotlin file "fixtures/kotlin/TokenService.kt"
    When I analyze the file for behaviors
    Then I should infer Result-based behaviors:
      | input           | output                               |
      | Valid JWT token | Result.success(TokenClaims)          |
      | Expired token   | Result.failure(TokenExpiredException) |
      | Invalid token   | Result.failure(InvalidTokenException) |

  # =============================================================================
  # Behavior Inference
  # =============================================================================

  Scenario: Infer success behavior from return statements
    Given a TypeScript file "fixtures/typescript/index.ts"
    When I analyze the file for behaviors
    Then I should infer success behaviors:
      | input           | output        |
      | Valid JWT token | Claims object |

  Scenario: Infer error behavior from try-catch blocks
    Given a TypeScript file "fixtures/typescript/index.ts"
    When I analyze the file for behaviors
    Then I should infer error behaviors:
      | input         | output            |
      | Expired token | TokenExpiredError |
      | Invalid token | InvalidTokenError |

  Scenario: Infer error behavior from Go error returns
    Given a Go file "fixtures/go/token.go"
    When I analyze the file for behaviors
    Then I should infer error behaviors:
      | input         | output          |
      | Expired token | ErrExpiredToken |
      | Invalid token | ErrInvalidToken |

  Scenario: Infer error behavior from Rust Result types
    Given a Rust file "fixtures/rust/lib.rs"
    When I analyze the file for behaviors
    Then I should infer error behaviors:
      | input         | output               |
      | Expired token | TokenError::Expired  |
      | Invalid token | TokenError::Invalid  |

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: Handle empty directory
    Given an empty directory "fixtures/empty"
    When I analyze the directory
    Then I should return an empty analysis result:
      | exports_count      | 0 |
      | dependencies_count | 0 |
      | behaviors_count    | 0 |

  Scenario: Handle file read failure gracefully
    Given a non-existent file "fixtures/missing/file.ts"
    When I attempt to analyze the file
    Then I should skip the file with a warning
    And the analysis should continue without error

  Scenario: Handle mixed language directory
    Given a directory with multiple languages
    When I analyze the directory
    Then I should detect and apply correct patterns per file extension

  # =============================================================================
  # Complete Analysis Output
  # =============================================================================

  Scenario: Generate complete TypeScript analysis JSON
    Given a TypeScript directory "fixtures/typescript"
    And a boundary file specifying direct_files: ["index.ts", "types.ts"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/typescript-analysis.json"
    And the result should include:
      | field              | expected_count |
      | exports.functions  | 2              |
      | exports.types      | 4              |
      | exports.classes    | 2              |
      | dependencies.external | 1           |
      | dependencies.internal | 2           |
      | behaviors          | 3              |
      | analyzed_files     | 2              |

  Scenario: Generate complete Python analysis JSON
    Given a Python directory "fixtures/python"
    And a boundary file specifying direct_files: ["__init__.py", "auth.py"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/python-analysis.json"

  Scenario: Generate complete Go analysis JSON
    Given a Go directory "fixtures/go"
    And a boundary file specifying direct_files: ["token.go"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/go-analysis.json"

  Scenario: Generate complete Rust analysis JSON
    Given a Rust directory "fixtures/rust"
    And a boundary file specifying direct_files: ["lib.rs"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/rust-analysis.json"

  Scenario: Generate complete Java analysis JSON
    Given a Java directory "fixtures/java"
    And a boundary file specifying direct_files: ["TokenService.java", "TokenClaims.java", "TokenConfig.java", "Role.java", "TokenExpiredException.java", "InvalidTokenException.java"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/java-analysis.json"
    And the result should include:
      | field              | expected_count |
      | exports.functions  | 2              |
      | exports.types      | 0              |
      | exports.classes    | 5              |
      | dependencies.external | 1           |
      | dependencies.internal | 0           |
      | behaviors          | 3              |
      | analyzed_files     | 6              |

  Scenario: Generate complete Kotlin analysis JSON
    Given a Kotlin directory "fixtures/kotlin"
    And a boundary file specifying direct_files: ["TokenService.kt", "TokenClaims.kt", "TokenConfig.kt", "Role.kt", "Exceptions.kt"]
    When I run the complete code-analyze workflow
    Then the output JSON should match "fixtures/expected/kotlin-analysis.json"
    And the result should include:
      | field              | expected_count |
      | exports.functions  | 2              |
      | exports.types      | 2              |
      | exports.classes    | 3              |
      | dependencies.external | 1           |
      | dependencies.internal | 0           |
      | behaviors          | 3              |
      | analyzed_files     | 5              |
