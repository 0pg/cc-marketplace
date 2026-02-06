Feature: State Diagram Generation
  As a specification author
  I want to generate Mermaid State diagrams from CLAUDE.md Protocol section
  So that I can visualize state machine behavior

  # =============================================================================
  # State Diagram Generation
  # =============================================================================

  Scenario: Generate State diagram from Protocol section
    Given a CLAUDE.md spec with protocol states "Idle", "Loading", "Loaded", "Error"
    And transitions:
      | from    | trigger   | to      |
      | Idle    | load()    | Loading |
      | Loading | success   | Loaded  |
      | Loading | failure   | Error   |
    When I generate the State diagram
    Then the output should contain "stateDiagram-v2"
    And the output should contain "[*] --> Idle"
    And the output should contain "Idle --> Loading : load()"
    And the output should contain "Loading --> Loaded : success"
    And the output should contain "Loading --> Error : failure"

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: No Protocol section returns None
    Given a CLAUDE.md spec with no Protocol section
    When I generate the State diagram
    Then no diagram should be generated

  Scenario: Empty Protocol returns None
    Given a CLAUDE.md spec with empty Protocol states and transitions
    When I generate the State diagram
    Then no diagram should be generated
