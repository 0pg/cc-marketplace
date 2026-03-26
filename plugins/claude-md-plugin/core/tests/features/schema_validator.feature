Feature: Schema Validation
  As a developer maintaining CLAUDE.md files
  I want to validate that they follow the required schema
  So that they can be reliably used as the primary SSOT

  Background:
    Given a clean test directory

  Scenario: Missing Purpose fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should fail
    And error should mention "Missing required section: Purpose"

  Scenario: Missing Requirements fails validation
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
    And error should mention "Missing required section: Requirements"

  Scenario: Missing Domain Context fails validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Validates authentication tokens.

      ## Requirements
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

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: Valid spec with requirements passes validation
    Given CLAUDE.md with content:
      """
      # Auth Module

      ## Purpose
      Validates authentication tokens.

      ## Requirements
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

      ## Requirements
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

      ## Requirements
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

      ## Requirements
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
    Then fix should add sections "Requirements, Domain Context"
    And the fixed file should pass validation

  Scenario: Fix schema does not modify complete files
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
    When I fix the schema
    Then fix should add sections ""
