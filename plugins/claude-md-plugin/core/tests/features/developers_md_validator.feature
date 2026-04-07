Feature: DEVELOPERS.md Schema Validation
  As a developer maintaining CLAUDE.md + DEVELOPERS.md file pairs
  I want to validate that DEVELOPERS.md follows the required schema
  So that documentation pairs remain consistent and useful

  Background:
    Given a clean test directory

  # INV-3: DEVELOPERS.md must exist alongside CLAUDE.md in strict mode
  Scenario: Missing DEVELOPERS.md warns in strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass
    And validation should have warnings
    And warning should mention "INV-3"

  Scenario: DEVELOPERS.md with all sections passes strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Decision Log
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: DEVELOPERS.md with only required sections passes strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: DEVELOPERS.md missing Constraints fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Missing required section: Constraints"

  Scenario: DEVELOPERS.md missing Technical Context fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Missing required section: Technical Context"

  Scenario: Constraints allows None
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Technical Context allows None
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Decision Log allows None
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Decision Log
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Non-strict mode does not check DEVELOPERS.md
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Decision Log with valid ADR entries passes
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Decision Log

      ### HMAC-SHA256 Selection
      - **Context**: Need a token verification method between internal services
      - **Decision**: Use HMAC-SHA256
      - **Rationale**: Internal services do not require RSA key management complexity
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: DEVELOPERS.md with Data Schemas section passes validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Data Schemas
      None

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Flows section in non-project-root DEVELOPERS.md generates warning
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Flows

      ### Login
      1. api/auth — POST /login
      """
    When I validate the schema with strict mode in non-project-root
    Then validation should pass
    And validation should have warnings
    And warning should mention "Flows"

  Scenario: Flows section in project-root DEVELOPERS.md does not generate warning
    Given CLAUDE.md with content:
      """
      # Project Root

      ## Purpose
      Project root module.

      ## Requirements
      None

      ## Domain Context
      None

      ## Instructions
      Follow project conventions.

      ## Conventions

      ### Project Structure
      Layered architecture.

      ### Module Boundaries
      Each module owns its data.

      ### Naming Conventions
      kebab-case directories.

      ### Language & Runtime
      TypeScript, Node.js 18+

      ### Coding Rules
      - Use async/await

      ### Naming Rules
      camelCase for variables.
      """
    And DEVELOPERS.md with content:
      """
      # Project Root

      ## Constraints
      None

      ## Technical Context
      None

      ## Flows

      ### User Login
      1. api/auth — POST /login
      2. domain/auth — validateCredentials() → Session
      """
    When I validate the schema with strict mode in project-root
    Then validation should pass
    And validation should have no warnings about "Flows"

  Scenario: Flows warning message mentions section name not hardcoded string
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Flows

      ### Login
      1. api/auth — POST /login
      """
    When I validate the schema with strict mode in non-project-root
    Then validation should pass
    And validation should have warnings
    And warning should mention "Flows"
    And warning should mention "is_project_root"
