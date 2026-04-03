Feature: Language Validation
  As a developer maintaining CLAUDE.md files
  I want to validate that documents are written in the declared language
  So that document language consistency is enforced

  Background:
    Given a clean test directory

  Scenario: English document passes English validation
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides JWT token-based authentication to verify user identity.

      ## Requirements

      - Valid JWT tokens pass through with decoded user information
      - Expired tokens return a 401 Unauthorized error

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 90

  Scenario: Korean document passes Korean validation
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증을 위한 JWT 토큰 기반 인증을 제공합니다.

      ## Requirements

      - 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과
      - 만료된 토큰은 401 Unauthorized 에러를 반환

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 70

  Scenario: Korean content in English-expected document fails
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides authentication.

      ## Requirements

      - 유효한 JWT 토큰이 포함된 요청은 통과
      - 만료된 토큰은 401 에러를 반환
      - 토큰 서명이 유효하지 않으면 거부

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "below_threshold"
    And non target lines should not be empty

  Scenario: Code blocks are excluded from character counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Handles user authentication for API requests.

      ## Requirements

      - Users authenticate via JWT tokens
      - Invalid tokens are rejected

      ## Domain Context

      ```typescript
      // 인증 미들웨어 설정
      const middleware = createAuthMiddleware();
      ```
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"

  Scenario: Heading lines are stripped from counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증 처리 모듈입니다.

      ## Requirements

      - JWT 토큰 검증 기능 제공
      - 만료 토큰 거부 처리

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 85

  Scenario: Insufficient content is skipped
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Auth

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then language result should be "skipped"

  Scenario: Unsupported language returns error
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Test module.

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "French" and threshold 70
    Then language validation should fail with "UnsupportedLanguage"

  Scenario: Threshold boundary — exactly 70% passes
    Given a markdown file "CLAUDE.md" with content at exactly 70 percent Latin
    When I validate language with expected "English" and threshold 70
    Then language result should be "pass"

  Scenario: Non-target line detection uses 50% per-line rule
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides authentication services for the platform.

      ## Requirements

      - Complies with 개인정보보호법 regulation for data handling
      - 유효한 JWT 토큰이 포함된 요청은 디코딩된 사용자 정보와 함께 통과합니다

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then non target lines should contain line 9
    And non target lines should not contain line 7

  Scenario: Script distribution is reported correctly
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      Provides JWT authentication for API requests.

      ## Requirements

      - Valid tokens pass through
      - 만료된 토큰은 거부

      ## Domain Context

      None
      """
    When I validate language with expected "English" and threshold 70
    Then script distribution should contain "Latin"
    And script distribution should contain "Hangul"

  Scenario: Japanese document with Hiragana and Kanji passes
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      ユーザー認証を処理するモジュールです。

      ## Requirements

      - 有効なトークンは通過する
      - 期限切れトークンは拒否される

      ## Domain Context

      None
      """
    When I validate language with expected "Japanese" and threshold 70
    Then language result should be "pass"

  Scenario: None markers are stripped from counting
    Given a markdown file "CLAUDE.md" with content:
      """
      ## Purpose

      사용자 인증을 위한 모듈입니다. 이 모듈은 JWT 토큰을 검증합니다.

      ## Requirements

      None

      ## Domain Context

      None
      """
    When I validate language with expected "Korean" and threshold 70
    Then language result should be "pass"
    And target percentage should be greater than 85
