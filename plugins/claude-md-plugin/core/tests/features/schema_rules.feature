Feature: Schema Rules SSOT
  As a developer maintaining CLAUDE.md files
  I want a single source of truth for schema validation rules
  So that rules are consistent across documentation and code

  Background:
    Given a schema validator is initialized

  Scenario: Required sections are defined from YAML SSOT
    When I check the required sections
    Then required sections should include:
      | Domain Context |
      | Purpose        |
      | Requirements   |

  Scenario: Valid CLAUDE.md with all required sections passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Missing Purpose section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Purpose"

  Scenario: Missing Requirements section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Requirements"

  Scenario: Missing Domain Context section fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Requirements
      None
      """
    When I validate the file
    Then validation should fail with error "MissingSection"
    And the error should mention "Domain Context"

  Scenario: Requirements section with "None" passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Domain Context section with "N/A" passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Requirements
      N/A

      ## Domain Context
      N/A
      """
    When I validate the file
    Then validation should pass

  Scenario: Requirements section with actual content passes validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      This module handles authentication.

      ## Requirements
      - Password reset must be within 90 days
      - Maximum 5 concurrent sessions

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should pass

  Scenario: Purpose section with "None" fails validation
    Given a CLAUDE.md file with content:
      """
      # Test Module

      ## Purpose
      None

      ## Requirements
      None

      ## Domain Context
      None
      """
    When I validate the file
    Then validation should fail with error "InvalidSectionContent"
    And the error should mention "Purpose"
