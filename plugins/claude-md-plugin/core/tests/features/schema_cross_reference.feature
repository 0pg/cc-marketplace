Feature: Schema Validation with Cross-Reference Resolution
  As a specification author
  I want broken cross-references to be detected
  So that I don't have dangling links

  Scenario: Valid cross-reference passes
    Given module "auth" exports "validateToken"
    And module "api" references "auth/CLAUDE.md#validateToken"
    When I validate "api/CLAUDE.md" with symbol index
    Then validation should pass

  Scenario: Unresolved cross-reference fails
    Given module "api" references "auth/CLAUDE.md#nonExistent"
    And no module exports "nonExistent"
    When I validate "api/CLAUDE.md" with symbol index
    Then validation should fail with error "UnresolvedCrossReference"
    And the error suggestion should mention "auth/CLAUDE.md"

  Scenario: Validation without index falls back to syntax check
    Given module "api" references "auth/CLAUDE.md#anything"
    When I validate "api/CLAUDE.md" without symbol index
    Then validation should pass (syntax only)
