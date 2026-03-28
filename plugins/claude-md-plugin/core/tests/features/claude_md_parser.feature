Feature: CLAUDE.md Parser
  As a primary SSOT system
  I want to parse CLAUDE.md files into structured specs
  So that I can extract purpose, requirements, and domain context

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

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have purpose "Handles user authentication and token validation."

  Scenario: Parse Requirements section with bullet list
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Requirements
      - Password reset must be within 90 days
      - Maximum 5 concurrent sessions

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have requirements count 2

  Scenario: Parse Requirements section with None value
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have no requirements

  Scenario: Parse Domain Context section
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Requirements
      None

      ## Domain Context
      JWT tokens with PCI-DSS compliance, 7 day expiry.
      Redis cache for authentication latency reduction.
      """
    When I parse the CLAUDE.md file
    Then the spec should have domain context containing "PCI-DSS"

  Scenario: Parse Domain Context with None value
    Given a CLAUDE.md file with content:
      """
      # auth-module

      ## Purpose
      Authentication module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have no domain context

  Scenario: Parse Instructions section
    Given a CLAUDE.md file with content:
      """
      # my-project

      ## Purpose
      My project root.

      ## Requirements
      None

      ## Domain Context
      None

      ## Instructions
      Always use TypeScript strict mode.
      Follow the team's code review process.
      """
    When I parse the CLAUDE.md file
    Then the spec should have instructions containing "TypeScript strict mode"

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: Fail fast on missing Purpose section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Purpose"

  Scenario: Fail fast on missing Requirements section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Purpose
      Some purpose description.

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Requirements"

  Scenario: Fail fast on missing Domain Context section
    Given a CLAUDE.md file with content:
      """
      # module-name

      ## Purpose
      Some purpose description.

      ## Requirements
      None
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section: Domain Context"

  Scenario: Fail fast on completely malformed CLAUDE.md
    Given a CLAUDE.md file with content:
      """
      This is not a valid CLAUDE.md file.
      It has no proper sections.
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "Missing required section"

  Scenario: Unrecognized section produces warning
    Given a CLAUDE.md file with content:
      """
      # test-module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None

      ## Exports
      - some old section
      """
    When I parse the CLAUDE.md file
    Then the spec should have warnings containing "Exports"

  Scenario: Parse minimal valid spec
    Given a CLAUDE.md file with content:
      """
      # test-module

      ## Purpose
      Test module.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then the spec should have purpose "Test module."
    And the spec should have no requirements
    And the spec should have no domain context
    And the spec should have no instructions

  Scenario: Purpose with None value fails parsing
    Given a CLAUDE.md file with content:
      """
      # test-module

      ## Purpose
      None

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I parse the CLAUDE.md file
    Then parsing should fail with error "does not allow 'None'"
