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
