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

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass
    And validation should have warnings
    And warning should mention "INV-3"

  Scenario: DEVELOPERS.md with all required sections passes strict validation
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: DEVELOPERS.md missing File Map fails validation
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Missing required section: File Map"

  Scenario: File Map allows None
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Domain Context allows None
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Invariants allows None
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
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

      ## Constraints
      None

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Operations allows None
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
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

      ## Constraints
      None

      ## Domain Context
      None
      """
    When I validate the schema
    Then validation should pass

  Scenario: DEVELOPERS.md missing Invariants fails validation
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Decision Log
      None

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Missing required section: Invariants"

  Scenario: Decision Log with valid ADR entries passes
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
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Domain Context
      None

      ## Invariants
      None

      ## Decision Log

      ### HMAC-SHA256 선택
      - **맥락**: 내부 서비스 간 토큰 검증 방식 필요
      - **결정**: HMAC-SHA256 사용
      - **근거**: 내부 서비스라 RSA 키 관리 복잡성 불필요

      ## Operations
      None

      ## File Map
      None
      """
    When I validate the schema with strict mode
    Then validation should pass
