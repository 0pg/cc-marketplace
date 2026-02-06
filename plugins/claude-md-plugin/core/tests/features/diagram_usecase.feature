Feature: UseCase Diagram Generation
  As a specification author
  I want to generate Mermaid UseCase diagrams from CLAUDE.md Behavior section
  So that I can visualize module interactions

  # =============================================================================
  # v2 Mode: Actors + Use Cases
  # =============================================================================

  Scenario: Generate UseCase diagram from v2 Behavior
    Given a CLAUDE.md spec with actors "User" and "System"
    And use case "UC-1" named "Token Validation" with actor "User"
    And use case "UC-2" named "Token Issuance" with actor "System"
    And use case "UC-1" includes "UC-3"
    When I generate the UseCase diagram
    Then the output should contain "flowchart LR"
    And the output should contain actor "User((User))"
    And the output should contain actor "System((System))"
    And the output should contain "UC_1[Token Validation]"
    And the output should contain "User --> UC_1"
    And the output should contain "UC_1 -.include.-> UC_3"

  # =============================================================================
  # v1 Fallback: Flat Behaviors
  # =============================================================================

  Scenario: Generate UseCase diagram from v1 flat behaviors
    Given a CLAUDE.md spec with behaviors:
      | input         | output             |
      | valid token   | Claims             |
      | expired token | TokenExpiredError  |
    And no actors or use cases defined
    When I generate the UseCase diagram
    Then the output should contain "flowchart LR"
    And the output should contain "User((User))"
    And the output should contain "UC1[valid token"
    And the output should contain "User --> UC1"

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: Empty behaviors produce minimal diagram
    Given a CLAUDE.md spec with no behaviors
    When I generate the UseCase diagram
    Then the output should contain "flowchart LR"
    And the output should have no use case nodes
