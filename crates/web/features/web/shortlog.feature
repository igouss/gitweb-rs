Feature: Serving a project's recent history (shortlog action)
  gitweb's git_shortlog opens the project, walks recent history from HEAD (or a
  named revision), and serves it as one row per commit: the commit's date, its
  author, its subject linking to the commit, and per-commit commit / commitdiff /
  tree links. An unborn repository (no HEAD) serves the page with no commits and
  does not fail; a request for a project that does not exist is die_error(404).

  Scenario: the shortlog page lists the project's recent commits
    Given a project root containing repository "repo.git"
    And the shortlog action is served
    When I GET "/?p=repo.git&a=shortlog"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Shortlog"
    And the response body contains "init"
    And the response body contains "Ada Lovelace"
    And the response body contains "commitdiff"
    And the response body contains "2023-11-14"

  Scenario: an unborn repository serves an empty shortlog page
    Given a repository "void.git" with an unborn HEAD
    And the shortlog action is served
    When I GET "/?p=void.git&a=shortlog"
    Then the response status is 200
    And the response body contains "No commits."

  Scenario: shortlog for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the shortlog action is served
    When I GET "/?p=ghost.git&a=shortlog"
    Then the response status is 404
