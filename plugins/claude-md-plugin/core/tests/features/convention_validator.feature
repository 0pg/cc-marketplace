Feature: Convention Validator
  As a developer using claude-md-plugin
  I want convention sections validated within CLAUDE.md
  So that project and code conventions are enforced consistently

  Background:
    Given a clean test directory

  # ---- Conventions (unified section) ----

  Scenario: Project root with valid Conventions section
    Given a project root with CLAUDE.md containing valid Conventions
    When I validate conventions
    Then convention validation should pass
    And conventions should be found

  Scenario: Project root missing Conventions section
    Given a project root with CLAUDE.md without Conventions
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Conventions"

  Scenario: Conventions missing required subsections
    Given a project root with CLAUDE.md containing incomplete Conventions
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Module Boundaries"

  # ---- Module Detection ----

  Scenario: Single module project auto-detection
    Given a single module project with package.json
    When I detect module roots
    Then module root count should be 1

  Scenario: Multi module project auto-detection
    Given a multi module project with sub-packages
    When I detect module roots
    Then module root count should be at least 2

  # ---- Module Override ----

  Scenario: Module root with Conventions override
    Given a multi module project with module-level Conventions override
    When I validate conventions
    Then convention validation should pass
    And module should have conventions override

  # ---- DRY: Convention Inheritance ----

  Scenario: Multi-module module without Conventions inherits from project root
    Given a multi module project where module has no Conventions
    When I validate conventions
    Then convention validation should pass

  Scenario: Multi-module module with malformed Conventions still fails
    Given a multi module project where module has incomplete Conventions
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Naming Rules"

  Scenario: Project root must have Conventions as canonical source
    Given a multi module project where project root has no Conventions
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Conventions"
