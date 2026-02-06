Feature: Dependency Graph Symbol Entries
  As a developer
  I want dependency graph nodes to contain symbol entries
  So that I can trace symbol-level dependencies

  Scenario: Nodes include typed symbol entries
    Given module "auth" exports function "validateToken" and type "Claims"
    When I build the dependency graph
    Then node "auth" should have 2 symbol entries
    And symbol entry "validateToken" should have kind "function"

  Scenario: Edges reference imported symbols
    Given module "auth" exports "validateToken"
    And module "api" depends on "auth" importing "validateToken"
    When I build the dependency graph
    Then the edge from "api" to "auth" should list "validateToken"

  # =============================================================================
  # CLAUDE.md Dependencies → Graph Edges
  # =============================================================================

  Scenario: CLAUDE.md Dependencies section creates graph edges
    Given module "src/auth" has Dependencies "internal: ../utils"
    And module "src/utils" exports "formatError"
    When I build the dependency graph
    Then edges should contain edge from "src/auth" to "src/utils"
    And the edge type should be "spec"

  Scenario: Both code and spec edges coexist
    Given module "src/auth" imports "../utils/format" in source code
    And module "src/auth" has Dependencies "internal: ../utils"
    When I build the dependency graph
    Then there should be at least 1 edge from "src/auth" to "src/utils"
