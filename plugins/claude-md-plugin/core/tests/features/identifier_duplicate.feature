Feature: Duplicate Identifier Detection (v17 P2-a)
  As a PM/PO running /validate --strict
  I want duplicate REQ-*/CONST-* identifiers detected
  So that spec documents cannot silently accumulate collisions across edits

  Background:
    Given a clean test directory

  Scenario: Duplicate CONST identifier in Constraints fails strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Demonstrates duplicate detection.

      ## Requirements
      - REQ-1: first requirement

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      - CONST-G-1: first group item
      - CONST-G-2: second group item
      - CONST-G-1: duplicate of first

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Duplicate identifier 'CONST-G-1'"

  Scenario: Prefix-aware match treats phase-prefixed IDs as distinct
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Demonstrates prefix-aware matching.

      ## Requirements
      - REQ-1: placeholder

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      - CONST-F-a-1: phase a constraint
      - CONST-F-c-1: phase c constraint

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Clean document with unique identifiers passes
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      All identifiers unique.

      ## Requirements
      - REQ-1: first
      - REQ-2: second

      ## Domain Context
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      - CONST-1: first
      - CONST-2: second
      - CONST-3: third

      ## Technical Context
      None
      """
    When I validate the schema with strict mode
    Then validation should pass

  Scenario: Duplicate REQ identifier in CLAUDE.md Requirements fails strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Duplicate REQ detection.

      ## Requirements
      - REQ-1: first
      - REQ-2: second
      - REQ-1: oops repeated

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
    Then validation should fail
    And error should mention "Duplicate identifier 'REQ-1'"
