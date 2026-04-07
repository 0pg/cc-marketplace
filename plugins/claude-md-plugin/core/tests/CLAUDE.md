# core/tests

## Purpose

Provides the BDD acceptance test infrastructure for the claude-md-plugin CLI core library. This directory is the living specification: every CLI subcommand's behavior is governed by Gherkin scenarios here, ensuring the Rust core stays consistent with its declared contracts across all supported languages.

## Requirements

- REQ-1: The test suite must cover all CLI subcommands exposed by `claude_md_core` (TreeParser, BoundaryResolver, SchemaValidator, CodeAnalyzer, ClaudeMdParser, ConventionValidator, CompileTargetResolver, ExportsFormatter, LanguageValidator).
- REQ-2: Each subcommand's happy path and key edge cases must be captured as independent, deterministic Cucumber scenarios.
- REQ-3: Fixtures for each supported language (Go, Java, Kotlin, Python, Rust, TypeScript) must exist under `fixtures/` so that language-specific analyzer scenarios can run against real source samples.
- REQ-4: All scenarios must pass without network access or external service dependencies.
- REQ-5: The `TestWorld` shared state must be reset between scenarios to guarantee test isolation.

## Domain Context

None
