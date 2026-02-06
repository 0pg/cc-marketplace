Feature: Symbol Index
  As a developer
  I want to build a symbol index from CLAUDE.md exports
  So that I can perform go-to-definition and find-references across modules

  Background:
    Given a project tree with multiple CLAUDE.md files

  # =============================================================================
  # Symbol Indexing
  # =============================================================================

  Scenario: Index symbols from multiple modules
    Given module "auth" exports function "validateToken"
    And module "auth" exports type "Claims"
    And module "config" exports variable "JWT_SECRET"
    When I build the symbol index
    Then the index should contain 3 symbols
    And symbol "validateToken" should have kind "function"
    And symbol "validateToken" should have anchor "auth/CLAUDE.md#validateToken"

  # =============================================================================
  # Go-to-Definition
  # =============================================================================

  Scenario: Find symbol by name (go-to-definition)
    Given module "auth" exports function "validateToken"
    And module "utils" exports function "hashPassword"
    When I build the symbol index
    And I find symbol "validateToken"
    Then I should get 1 result
    And the result should point to module "auth"

  Scenario: Find symbol with multiple definitions
    Given module "auth" exports function "validate"
    And module "input" exports function "validate"
    When I build the symbol index
    And I find symbol "validate"
    Then I should get 2 results

  # =============================================================================
  # Cross-Reference Detection
  # =============================================================================

  Scenario: Detect valid cross-reference
    Given module "auth" exports function "validateToken"
    And module "api" references "auth/CLAUDE.md#validateToken" in Purpose section
    When I build the symbol index
    Then the index should have 1 reference
    And the reference should be valid

  Scenario: Detect unresolved cross-reference
    Given module "api" references "nonexistent/CLAUDE.md#missingSymbol" in Purpose section
    When I build the symbol index
    Then the index should have 1 unresolved reference
    And the unresolved reference should target "missingSymbol"

  # =============================================================================
  # Find References
  # =============================================================================

  Scenario: Find all references to a symbol
    Given module "auth" exports function "validateToken"
    And module "api" references "auth/CLAUDE.md#validateToken"
    And module "middleware" references "auth/CLAUDE.md#validateToken"
    When I build the symbol index
    And I find references to "auth/CLAUDE.md#validateToken"
    Then I should get 2 references
