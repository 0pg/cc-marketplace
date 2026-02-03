Feature: Schema Rules SSOT
  As a developer maintaining CLAUDE.md files
  I want a single source of truth for schema validation rules
  So that rules are consistent across documentation and code

  Background:
    Given a schema validator is initialized

  Scenario: Required sections are defined from YAML SSOT
    When I check the required sections
    Then required sections should include:
      | Purpose  |
      | Exports  |
      | Behavior |
      | Contract |
      | Protocol |

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

      ## Contract
      None

      ## Protocol
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

      ## Contract
      None

      ## Protocol
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

      ## Protocol
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Contract"

  Scenario: Missing Protocol section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Contract
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Protocol"

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

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Protocol section with "N/A" passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Exports
      - `validate(token: string): boolean`

      ## Behavior
      - valid token → true

      ## Contract
      N/A

      ## Protocol
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

      ## Contract

      ### validate
      - **Preconditions**: token must be non-empty string
      - **Postconditions**: returns boolean

      ## Protocol
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

      ## Contract
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
