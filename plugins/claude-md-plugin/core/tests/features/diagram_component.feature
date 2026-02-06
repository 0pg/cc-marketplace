Feature: Component Diagram Generation
  As a project architect
  I want to generate Mermaid Component diagrams from dependency graph
  So that I can visualize module relationships

  # =============================================================================
  # Component Diagram Generation
  # =============================================================================

  Scenario: Generate Component diagram from dependency graph
    Given a dependency graph with modules:
      | path   | exports                    |
      | auth   | validateToken, Claims      |
      | config | JWT_SECRET                 |
    And dependency edges:
      | from | to     | imported_symbols |
      | auth | config | JWT_SECRET       |
    When I generate the Component diagram
    Then the output should contain "flowchart TB"
    And the output should contain subgraph "auth"
    And the output should contain export node "validateToken"
    And the output should contain "auth -->|JWT_SECRET| config"

  # =============================================================================
  # Edge Cases
  # =============================================================================

  Scenario: Module without exports shows empty subgraph
    Given a dependency graph with modules:
      | path  | exports |
      | utils |         |
    When I generate the Component diagram
    Then the output should contain subgraph "utils"
    And the subgraph should have no export nodes

  Scenario: Multiple dependency edges between modules
    Given a dependency graph with modules:
      | path   | exports           |
      | auth   | validateToken     |
      | config | JWT_SECRET, PORT  |
    And dependency edges:
      | from | to     | imported_symbols   |
      | auth | config | JWT_SECRET, PORT   |
    When I generate the Component diagram
    Then the output should contain "auth -->|JWT_SECRET, PORT| config"
