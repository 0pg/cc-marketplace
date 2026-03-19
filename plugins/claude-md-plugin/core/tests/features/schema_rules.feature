Feature: Schema Rules SSOT
  As a developer maintaining CLAUDE.md files
  I want a single source of truth for schema validation rules
  So that rules are consistent across documentation and code

  Background:
    Given a schema validator is initialized

  Scenario: Required sections are defined from YAML SSOT
    When I check the required sections
    Then required sections should include:
      | Purpose        |
      | Exports        |
      | Behavior       |
      | Domain Context |
      | Contract       |
      | Error Taxonomy |

  Scenario: Valid CLAUDE.md with all required sections passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true
      - invalid token → false

      ## Domain Context
      None

      ## Contract
      None

      ## Error Taxonomy
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Missing Purpose section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract
      None

      ## Error Taxonomy
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Purpose"

  Scenario: Missing Contract section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Error Taxonomy
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Contract"

  Scenario: Missing Error Taxonomy section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Error Taxonomy"

  Scenario: Contract section with "None" passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract
      None

      ## Error Taxonomy
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Error Taxonomy section with "N/A" passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract
      N/A

      ## Error Taxonomy
      N/A
      """
    When I validate the file
    Then validation should pass

  Scenario: Contract section with actual content passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract

      ### validate
      - **Preconditions**: token must be non-empty string
      - **Postconditions**: returns boolean

      ## Error Taxonomy
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Protocol section with state machine passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Domain Context
      None

      ## Contract
      None

      ## Error Taxonomy
      None

      ## Protocol

      ### State Machine
      States: `Idle` | `Validating` | `Done`

      Transitions:
      - `Idle` + `validate()` → `Validating`
      - `Validating` + `success` → `Done`
      """
    When I validate the file
    Then validation should pass
