Feature: Format Exports
  As a decompiler or validator agent
  I want to convert analyze-code JSON exports into deterministic CLAUDE.md Exports markdown
  So that Exports sections are consistent and free from LLM generation errors

  # =============================================================================
  # Empty Exports
  # =============================================================================

  Scenario: Empty exports produce None
    Given an analyze-code JSON with no exports
    When I format the exports
    Then the formatted output should be "None"

  # =============================================================================
  # Single Category (flat list, no subsection)
  # =============================================================================

  Scenario: Single function produces flat list
    Given an analyze-code JSON with exports:
      | category  | name   | signature              |
      | function  | greet  | greet(name: string): string |
    When I format the exports
    Then the formatted output should be:
      """
      - `greet(name: string): string`
      """

  Scenario: Multiple functions sorted alphabetically
    Given an analyze-code JSON with exports:
      | category  | name   | signature       |
      | function  | zebra  | zebra(): void   |
      | function  | alpha  | alpha(): void   |
      | function  | middle | middle(): void  |
    When I format the exports
    Then the formatted output should be:
      """
      - `alpha(): void`
      - `middle(): void`
      - `zebra(): void`
      """

  Scenario: Single type with definition
    Given an analyze-code JSON with exports:
      | category | name   | definition      |
      | type     | Config | timeout: number |
    When I format the exports
    Then the formatted output should be:
      """
      - `Config { timeout: number }`
      """

  Scenario: Single enum with variants
    Given an analyze-code JSON with exports:
      | category | name   | variants           |
      | enum     | Status | Active,Inactive,Pending |
    When I format the exports
    Then the formatted output should be:
      """
      - `Status: Active | Inactive | Pending`
      """

  Scenario: Single variable with type
    Given an analyze-code JSON with exports:
      | category | name        | var_type |
      | variable | MAX_RETRIES | number   |
    When I format the exports
    Then the formatted output should be:
      """
      - `MAX_RETRIES: number`
      """

  Scenario: Single class with signature
    Given an analyze-code JSON with exports:
      | category | name         | signature                    |
      | class    | TokenManager | TokenManager(secret: string) |
    When I format the exports
    Then the formatted output should be:
      """
      - `TokenManager(secret: string)`
      """

  Scenario: Single re-export with source
    Given an analyze-code JSON with exports:
      | category  | name   | source   |
      | re_export | Logger | ./logger |
    When I format the exports
    Then the formatted output should be:
      """
      - `Logger` (from `./logger`)
      """

  # =============================================================================
  # Multiple Categories (subsection headers)
  # =============================================================================

  Scenario: Two categories produce subsection headers
    Given an analyze-code JSON with exports:
      | category | name   | signature            | definition      |
      | function | run    | run(): void          |                 |
      | type     | Config |                      | timeout: number |
    When I format the exports
    Then the formatted output should contain subsection "### Functions"
    And the formatted output should contain subsection "### Types"
    And the formatted output should not contain subsection "### Classes"

  Scenario: Category order is fixed regardless of input order
    Given an analyze-code JSON with exports:
      | category  | name     | signature   | source   | variants | var_type | definition |
      | re_export | reExp    |             | ./mod    |          |          |            |
      | variable  | VAR      |             |          |          | string   |            |
      | enum      | Dir      |             |          | Up,Down  |          |            |
      | function  | run      | run(): void |          |          |          |            |
    When I format the exports
    Then the subsection order should be:
      | subsection     |
      | Functions      |
      | Enums          |
      | Variables      |
      | Re-exports     |

  # =============================================================================
  # Determinism
  # =============================================================================

  Scenario: Same input produces identical output on multiple runs
    Given an analyze-code JSON with exports:
      | category | name  | signature            | definition      |
      | function | beta  | beta(): void         |                 |
      | function | alpha | alpha(): void        |                 |
      | type     | Zeta  |                      | x: number       |
      | type     | Alpha |                      |                 |
    When I format the exports twice
    Then both outputs should be identical
