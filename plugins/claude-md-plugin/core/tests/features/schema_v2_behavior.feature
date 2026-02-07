Feature: Schema v2 Behavior Section
  As a specification author
  I want to define Actors and Use Cases in the Behavior section
  So that UseCase diagrams can be generated deterministically

  Background:
    Given the parser supports schema version 2.0
    And the validator detects v2 via "<!-- schema: 2.0 -->" marker

  # =============================================================================
  # Actor Parsing
  # =============================================================================

  Scenario: Parse Actors subsection
    Given a v2 CLAUDE.md file with content:
      """
      <!-- schema: 2.0 -->
      # auth

      ## Purpose
      Authentication module.

      ## Summary
      Auth module summary.

      ## Exports
      None

      ## Behavior

      ### Actors
      - User: End user needing authentication
      - System: Internal token verification system

      ### UC-1: Token Validation
      - Actor: User
      - valid token → Claims

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have actors:
      | name   | description                         |
      | User   | End user needing authentication     |
      | System | Internal token verification system  |

  # =============================================================================
  # UseCase Parsing
  # =============================================================================

  Scenario: Parse Use Cases with Actor, Includes, Extends
    Given a v2 CLAUDE.md file with content:
      """
      <!-- schema: 2.0 -->
      # auth

      ## Purpose
      Authentication module.

      ## Summary
      Auth module summary.

      ## Exports
      None

      ## Behavior

      ### Actors
      - User: End user

      ### UC-1: Token Validation
      - Actor: User
      - valid token → Claims
      - expired token → TokenExpiredError
      - Includes: UC-3

      ### UC-2: Token Issuance
      - Actor: System
      - user info → signed JWT
      - Extends: UC-1

      ### UC-3: Token Parsing
      - Actor: System
      - JWT string → Header + Payload

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have use cases:
      | id   | name             | actor  |
      | UC-1 | Token Validation | User   |
      | UC-2 | Token Issuance   | System |
      | UC-3 | Token Parsing    | System |
    And use case "UC-1" should include "UC-3"
    And use case "UC-2" should extend "UC-1"

  # =============================================================================
  # Schema Version Detection
  # =============================================================================

  Scenario: Detect schema version 2.0 from marker
    Given a v2 CLAUDE.md file with content:
      """
      <!-- schema: 2.0 -->
      # test-module

      ## Purpose
      Test module.

      ## Summary
      Summary.

      ## Exports
      None

      ## Behavior
      - input → output

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have schema version "2.0"

  Scenario: v1 file has no schema version
    Given a CLAUDE.md file with content:
      """
      # test-module

      ## Purpose
      Test module.

      ## Summary
      Summary.

      ## Exports
      None

      ## Behavior
      - input → output

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have no schema version

  # =============================================================================
  # v2 Validation
  # =============================================================================

  Scenario: Duplicate UC-ID fails validation
    Given a v2 CLAUDE.md file with duplicate UC-1 IDs
    When I validate the schema
    Then validation should fail with error "DuplicateUseCaseId"

  Scenario: Invalid Include target fails validation
    Given a v2 CLAUDE.md file with Includes referencing non-existent UC-99
    When I validate the schema
    Then validation should fail with error "InvalidIncludeTarget"

  Scenario: Valid v2 file passes validation
    Given a v2 CLAUDE.md file with valid Actors and Use Cases
    When I validate the schema
    Then validation should pass
