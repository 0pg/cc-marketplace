Feature: Schema Validation
  As a developer maintaining CLAUDE.md files
  I want to validate that they follow the required schema
  So that they can be reliably used as pre-learning indices

  Background:
    Given a clean test directory

  Scenario: Missing Purpose fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Purpose"

  Scenario: Missing Constraints fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Validates authentication tokens.

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Constraints"

  Scenario: Missing Domain Context fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Validates authentication tokens.

      ## Constraints
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Domain Context"

  Scenario: Valid minimal spec passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid spec with constraints passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Constraints
      - Token expiry must be 7 days (PCI-DSS)
      - Maximum 5 concurrent sessions

      ## Domain Context
      JWT tokens with RS256 algorithm.
      """
    When I validate the schema
    Then validation should pass

  Scenario: Purpose with None value fails validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      None

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "does not allow 'None'"

  Scenario: Domain Context with N/A passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Constraints
      N/A

      ## Domain Context
      N/A
      """
    When I validate the schema
    Then validation should pass

  Scenario: Unrecognized section produces warning
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Constraints
      None

      ## Domain Context
      None

      ## Exports
      - some old section
      """
    When I validate the schema
    Then validation should pass
    And validation should have warnings

  # Fix schema: auto-fix missing allow-none sections
  Scenario: Fix schema adds missing allow-none sections
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.
      """
    When I fix the schema
    Then fix should add sections "Constraints, Domain Context"
    And the fixed file should pass validation

  Scenario: Fix schema does not modify complete files
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I fix the schema
    Then fix should add sections ""
