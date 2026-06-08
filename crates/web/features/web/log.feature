Feature: Serving a project's verbose log (log action)
  gitweb's git_log opens the project, walks recent history from HEAD (or a named
  revision), and serves it verbose: a block per commit carrying the commit's
  relative age and subject, an authorship line with the author and the absolute
  date, the message body, and per-commit commit / commitdiff / tree links. An
  unborn repository (no HEAD) serves the page with no commits and does not fail; a
  request for a project that does not exist is die_error(404).

  Scenario: the log page shows the project's recent commits as blocks
    Given a project root containing repository "repo.git"
    And the log action is served
    When I GET "/?p=repo.git&a=log"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Log"
    And the response body contains "init"
    And the response body contains "Ada Lovelace"
    And the response body contains "commitdiff"
    And the response body contains "Tue, 14 Nov 2023 22:13:20 +0000"

  Scenario: an unborn repository serves an empty log page
    Given a repository "void.git" with an unborn HEAD
    And the log action is served
    When I GET "/?p=void.git&a=log"
    Then the response status is 200
    And the response body contains "No commits."

  Scenario: log for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the log action is served
    When I GET "/?p=ghost.git&a=log"
    Then the response status is 404
