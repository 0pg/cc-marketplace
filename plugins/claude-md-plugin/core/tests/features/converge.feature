Feature: Schema Convergence (fix-schema converge)
  The converge_schema function deterministically migrates DEVELOPERS.md
  to the current schema by applying renames, removals, and additions
  declared in schema-rules.yaml.

  Background:
    Given a clean test directory

  # Step 1: Renames

  Scenario: Rename section when only old name exists
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Invariants
      - INV-1: All outputs must be JSON

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should contain "## Constraints"
    And converged content should not contain "## Invariants"
    And converge changes should contain "renamed: ## Invariants"

  Scenario: Rename conflict when both old and new names exist
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Invariants
      - Old invariant content

      ## Constraints
      - New constraint content

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should contain "## Invariants"
    And converged content should contain "## Constraints"
    And converge warnings should contain "conflict"

  # Step 2: Removals

  Scenario: Remove Operations section from DEVELOPERS.md
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Operations
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should not contain "## Operations"
    And converge changes should contain "removed: ## Operations"

  Scenario: Remove Public API section from DEVELOPERS.md
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## Public API
      | Symbol | Signature | Called by |
      | foo | fn foo() | bar |
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should not contain "## Public API"
    And converge changes should contain "removed: ## Public API"

  Scenario: Remove File Map section from DEVELOPERS.md
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None

      ## File Map
      - src/lib.rs: main library
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should not contain "## File Map"
    And converge changes should contain "removed: ## File Map"

  # Step 3-4: Add missing sections

  Scenario: Add missing required section with None placeholder
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should contain "## Constraints"
    And converge changes should contain "added: ## Constraints (None)"

  Scenario: Add missing optional Data Schemas section
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should contain "## Data Schemas"
    And converge changes should contain "added: ## Data Schemas (None)"

  # Step 5: Conditional sections

  Scenario: Add Flows section at project root
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema at project root
    Then converged content should contain "## Flows"
    And converge changes should contain "added: ## Flows"

  Scenario: Do not add Flows at non-project-root
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Technical Context
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converged content should not contain "## Flows"

  # Idempotent

  Scenario: Already-current schema produces no changes
    Given DEVELOPERS.md with content:
      """
      # Test Module

      ## Constraints
      None

      ## Data Schemas
      None

      ## Technical Context
      None

      ## Decision Log
      None
      """
    When I converge the DEVELOPERS.md schema
    Then converge should report no changes
