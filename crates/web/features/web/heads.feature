Feature: Serving a project's branches (heads action)
  gitweb's git_heads page opens the project, lists its branches with their ages
  and per-branch shortlog / log / tree links, marks the branch HEAD points at,
  and serves it as an HTML page. A project with no branches lists nothing without
  failing, and a request for a project that does not exist is gitweb's
  die_error(404).

  Scenario: the heads page lists the project's branches
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the repository "repo.git" has branch "topic" committed at 2000
    And the heads action is served
    When I GET "/?p=repo.git&a=heads"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "main"
    And the response body contains "topic"
    And the response body contains "shortlog"
    And the response body contains "Heads"

  Scenario: the branch HEAD points at is marked current
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the repository "repo.git" has branch "topic" committed at 2000
    And the heads action is served
    When I GET "/?p=repo.git&a=heads"
    Then the response status is 200
    And the response body contains "current"

  Scenario: the heads page lists a branch under an extra branch directory
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the repository "repo.git" has a ref "refs/sandbox/wip" committed at 2000
    And the heads action is served with extra branch refs "sandbox"
    When I GET "/?p=repo.git&a=heads"
    Then the response status is 200
    And the response body contains "wip (sandbox)"
    And the response body contains "main"

  Scenario: a repository with no branches serves an empty heads page
    Given a repository "void.git" with an unborn HEAD
    And the heads action is served
    When I GET "/?p=void.git&a=heads"
    Then the response status is 200
    And the response body contains "No branches."

  Scenario: heads for a project that does not exist is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the heads action is served
    When I GET "/?p=ghost.git&a=heads"
    Then the response status is 404
