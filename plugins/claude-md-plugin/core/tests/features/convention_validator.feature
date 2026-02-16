Feature: Convention Validator
  As a developer using claude-md-plugin
  I want convention sections validated within CLAUDE.md
  So that project and code conventions are enforced consistently

  Background:
    Given a clean test directory

  # ---- Project Convention ----

  Scenario: Project root with valid Project Convention section
    Given a project root with CLAUDE.md containing valid Project Convention
    When I validate conventions
    Then convention validation should pass
    And project convention should be found

  Scenario: Project root missing Project Convention section
    Given a project root with CLAUDE.md without Project Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Project Convention"

  Scenario: Project Convention missing required subsections
    Given a project root with CLAUDE.md containing incomplete Project Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Module Boundaries"

  # ---- Code Convention ----

  Scenario: Module root with valid Code Convention section
    Given a project root with CLAUDE.md containing valid conventions
    When I validate conventions
    Then convention validation should pass
    And code convention should be found

  Scenario: Module root missing Code Convention section
    Given a project root with CLAUDE.md containing only Project Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Code Convention"

  Scenario: Code Convention missing required subsections
    Given a project root with CLAUDE.md containing incomplete Code Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Naming Rules"

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

  Scenario: Module root with Project Convention override
    Given a multi module project with module-level Project Convention override
    When I validate conventions
    Then convention validation should pass
    And module should have project convention override

  # ---- DRY: Convention Inheritance ----

  Scenario: Multi-module module without Code Convention inherits from project root
    Given a multi module project where module has no Code Convention
    When I validate conventions
    Then convention validation should pass

  Scenario: Multi-module module with malformed Code Convention still fails
    Given a multi module project where module has incomplete Code Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Naming Rules"

  Scenario: Project root must have Code Convention as canonical source
    Given a multi module project where project root has no Code Convention
    When I validate conventions
    Then convention validation should fail
    And convention error should mention "Code Convention"
