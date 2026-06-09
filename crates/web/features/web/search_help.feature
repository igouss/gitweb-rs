Feature: Serving the search-help page (search_help action)
  gitweb's git_search_help serves the search-syntax help as an HTML page. It is
  always available — there is no search-feature gate on the page itself — but it
  is a per-project action, so a request for a project that does not exist is a
  404. The grep and pickaxe entries are documented only when those features are
  enabled (they are, by default), so disabling them serves a page without their
  descriptions.

  Scenario: the help page documents every search type by default
    Given a repository "repo.git" with an unborn HEAD
    And the search_help action is served
    When I GET "/?p=repo.git&a=search_help"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "The commit messages and authorship information will be scanned"
    And the response body contains "All files in the currently selected tree"
    And the response body contains "appear or disappear from any file"

  Scenario: disabling grep and pickaxe omits their entries
    Given a repository "repo.git" with an unborn HEAD
    And the search_help action is served with grep and pickaxe disabled
    When I GET "/?p=repo.git&a=search_help"
    Then the response status is 200
    And the response body contains "The commit messages and authorship information will be scanned"
    And the response body does not contain "All files in the currently selected tree"
    And the response body does not contain "appear or disappear from any file"

  Scenario: search help for a project that does not exist is not found
    Given a repository "repo.git" with an unborn HEAD
    And the search_help action is served
    When I GET "/?p=ghost.git&a=search_help"
    Then the response status is 404
