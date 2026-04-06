Feature: Agent Observations in DEVELOPERS.md
  As a node-resident agent working on a module
  I want to record observations in DEVELOPERS.md's Agent Observations section
  So that experiential knowledge persists across sessions and aids future work

  Background:
    Given a clean test directory

  # --- Schema Validation ---

  Scenario: DEVELOPERS.md with Agent Observations section passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Requirements
      - REQ-1: JWT token validation

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Auth Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Agent Observations

      ### [structural] auth-utils circular import
      - anchor: REQ-1
      - since: 2026-03-15
      - refs: 3
      - source: /dev green-coder
      - auth -> utils -> auth cycle. Use type-only import.
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Agent Observations allows None
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Auth Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Agent Observations
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Agent Observations is optional
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Requirements
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Auth Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  # --- Entry Format Validation ---

  Scenario: Agent Observations entry with valid type tag passes
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

      ## Agent Observations

      ### [structural] known issue
      - since: 2026-03-15
      - refs: 1
      - source: /dev green-coder
      - Description of the observation.
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Agent Observations entry with invalid type tag warns
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

      ## Agent Observations

      ### [unknown_type] some entry
      - since: 2026-03-15
      - refs: 1
      - source: /dev green-coder
      - Some content.
      """
    When I validate the schema with strict mode
    Then validation should pass
    And validation should have warnings
    And warning should mention "unknown_type"

  Scenario: Agent Observations entry missing required fields warns
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

      ## Agent Observations

      ### [structural] missing fields entry
      - Some content without required metadata.
      """
    When I validate the schema with strict mode
    Then validation should pass
    And validation should have warnings
    And warning should mention "since"

  Scenario: All four entry types are valid
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

      ## Agent Observations

      ### [structural] architecture pattern
      - since: 2026-03-15
      - refs: 3
      - source: /dev green-coder
      - Observation about structure.

      ### [decision] tech choice
      - anchor: CONST-1
      - since: 2026-03-18
      - refs: 1
      - source: /spec impl
      - Decision rationale.

      ### [tactical] temp workaround
      - since: 2026-03-20
      - refs: 0
      - source: /dev refactorer
      - Short-lived note.

      ### [preference] coding style
      - since: 2026-03-22
      - refs: 5
      - source: /spec impl
      - User prefers functional style.
      """
    When I validate the schema with strict mode
    Then validation should pass

  # --- Converge Schema ---

  Scenario: Converge does not auto-add Agent Observations
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should not contain "Agent Observations"

  # --- Unrecognized Section ---

  Scenario: Agent Observations is recognized and does not produce unrecognized warning
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

      ## Agent Observations
      None
      """
    When I validate the schema with strict mode
    Then validation should pass
    And validation should have no warnings about "Agent Observations"
