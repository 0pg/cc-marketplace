Feature: DEVELOPERS.md Schema Validation
  As a developer maintaining CLAUDE.md + DEVELOPERS.md file pairs
  I want to validate that DEVELOPERS.md follows the required schema
  So that documentation pairs remain consistent and useful

  Background:
    Given a clean test directory

  # INV-3: DEVELOPERS.md must exist alongside CLAUDE.md in strict mode
  Scenario: Missing DEVELOPERS.md fails strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "INV-3"

  Scenario: DEVELOPERS.md with all required sections passes strict validation
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map

      | 파일 | 역할 | 의존 |
      |------|------|------|
      | index.ts | 진입점 | - |

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
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

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "Missing required section: File Map"

  Scenario: File Map does not allow None
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map
      None

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
      None
      """
    When I validate the schema with strict mode
    Then validation should fail
    And error should mention "File Map"
    And error should mention "does not allow 'None'"

  Scenario: Data Structures allows None
    Given CLAUDE.md with content:
      """
      # Test Module

      ## Purpose
      Test module.

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map

      | 파일 | 역할 | 의존 |
      |------|------|------|
      | index.ts | 진입점 | - |

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
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

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map

      | 파일 | 역할 | 의존 |
      |------|------|------|
      | index.ts | 진입점 | - |

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
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

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map

      | 파일 | 역할 | 의존 |
      |------|------|------|
      | index.ts | 진입점 | - |

      ## Data Structures
      None

      ## Decision Log
      None

      ## Operations
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

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
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

      ## Exports
      - `foo(x: int): string`

      ## Behavior
      - input → output

      ## Domain Context
      None

      ## Contract
      None

      ## Protocol
      None
      """
    And DEVELOPERS.md with content:
      """
      # Test Module

      ## File Map

      | 파일 | 역할 | 의존 |
      |------|------|------|
      | index.ts | 진입점 | - |

      ## Data Structures
      None

      ## Decision Log

      ### HMAC-SHA256 선택
      - **맥락**: 내부 서비스 간 토큰 검증 방식 필요
      - **결정**: HMAC-SHA256 사용
      - **근거**: 내부 서비스라 RSA 키 관리 복잡성 불필요

      ## Operations
      None
      """
    When I validate the schema with strict mode
    Then validation should pass
