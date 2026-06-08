Feature: A project's README.html through the gix ProjectStore adapter
  gitweb's summary page injects the bytes of `$GIT_DIR/README.html` verbatim,
  but only when the file exists and is non-empty (its `-s` test). This is the
  gix adapter reading that raw-HTML file over real repositories on disk: it
  hands back the contents when present and non-empty, and nothing otherwise. The
  prevent_xss gate that decides whether to USE the result is the caller's job,
  not the adapter's — the adapter only reads.

  The bytes are returned unescaped, as gitweb's `insert_file` emits them.

  # --- present / absent / empty (zero, one) ---

  Scenario: a non-empty README.html is read verbatim
    Given a project root containing repository "solo.git"
    And "solo.git" has the README.html "<h1>Hello</h1>"
    When I read the README of "solo.git"
    Then the README is "<h1>Hello</h1>"

  Scenario: a repository with no README.html has no README
    Given a project root containing repository "solo.git"
    When I read the README of "solo.git"
    Then there is no README

  Scenario: an empty README.html is treated as absent
    Given a project root containing repository "solo.git"
    And "solo.git" has the README.html ""
    When I read the README of "solo.git"
    Then there is no README

  # --- the missing and unsafe edges, shared with open() ---

  Scenario: reading the README for a name that does not exist fails as not found
    Given an empty project root
    When I read the README of "ghost.git"
    Then reading the README fails as not found

  Scenario: a path-traversal name is rejected as invalid
    Given an empty project root
    When I read the README of "../etc"
    Then reading the README fails as invalid
