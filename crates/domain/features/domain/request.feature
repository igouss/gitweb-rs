Feature: Request parameter parsing and validation
  gitweb maps short CGI names (p, a, h, hb, f, pg, s, st, opt, ...) onto its
  long-named request fields and validates each one before touching a
  repository — evaluate_and_validate_params. Malformed input is rejected up
  front with gitweb's status: a bad action / hash / file / page / searchtype
  is 400, a bad project is 404, and a too-short search term is 403.

  Project existence and export rules are the store's job; here we only reject
  what is unsafe on its face (path traversal) and what is malformed.

  Scenario: A bare request carries no action and no project
    Given a bare request
    When I parse the request parameters
    Then the request parses
    And the request has no action
    And the request has no project

  Scenario: The action short name resolves to an action
    Given a request parameter "a" is "summary"
    When I parse the request parameters
    Then the request parses
    And the request action is "summary"

  Scenario: A blob view carries its base ref and file path
    Given a request parameter "a" is "blob"
    And a request parameter "hb" is "master"
    And a request parameter "f" is "src/main.rs"
    When I parse the request parameters
    Then the request parses
    And the request action is "blob"
    And the request hash base is "master"
    And the request file name is "src/main.rs"
    And the request project is absent

  Scenario: A full object id is accepted as a hash
    Given a request parameter "h" is "0123456789abcdef0123456789abcdef01234567"
    When I parse the request parameters
    Then the request parses
    And the request hash is "0123456789abcdef0123456789abcdef01234567"

  Scenario: A project path is kept as named
    Given a request parameter "p" is "group/repo.git"
    When I parse the request parameters
    Then the request parses
    And the request project is "group/repo.git"

  Scenario: An unknown action is rejected as invalid
    Given a request parameter "a" is "wibble"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: A project that escapes the root is rejected as not found
    Given a request parameter "p" is "../secret"
    When I parse the request parameters
    Then the request is rejected as not found

  Scenario: A project filter that escapes the root is rejected as not found
    Given a request parameter "pf" is "../secret"
    When I parse the request parameters
    Then the request is rejected as not found

  Scenario: A file path that escapes the tree is rejected as invalid
    Given a request parameter "f" is "../etc/passwd"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: A ref with a forbidden byte is rejected as invalid
    Given a request parameter "hb" is "bad:ref"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: A numeric page is parsed
    Given a request parameter "pg" is "3"
    When I parse the request parameters
    Then the request parses
    And the request page is 3

  Scenario: A non-numeric page is rejected as invalid
    Given a request parameter "pg" is "12x"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: A lowercase searchtype is accepted
    Given a request parameter "st" is "author"
    When I parse the request parameters
    Then the request parses
    And the request search type is "author"

  Scenario: A searchtype with uppercase is rejected as invalid
    Given a request parameter "st" is "Author"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: A search term of two characters is accepted
    Given a request parameter "s" is "ab"
    When I parse the request parameters
    Then the request parses
    And the request search text is "ab"

  Scenario: A one-character search term is rejected as forbidden
    Given a request parameter "s" is "a"
    When I parse the request parameters
    Then the request is rejected as forbidden

  Scenario: An empty search term is rejected as forbidden
    Given a request parameter "s" is ""
    When I parse the request parameters
    Then the request is rejected as forbidden

  Scenario: --no-merges is allowed for the log action
    Given a request parameter "a" is "log"
    And a request parameter "opt" is "--no-merges"
    When I parse the request parameters
    Then the request parses
    And the request keeps the extra option "--no-merges"

  Scenario: --no-merges is not allowed for the summary action
    Given a request parameter "a" is "summary"
    And a request parameter "opt" is "--no-merges"
    When I parse the request parameters
    Then the request is rejected as invalid

  Scenario: An unknown extra option is rejected as invalid
    Given a request parameter "a" is "log"
    And a request parameter "opt" is "--reverse"
    When I parse the request parameters
    Then the request is rejected as invalid
