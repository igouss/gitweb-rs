Feature: The composition root serves a wired gitweb-rs over a real project root
  The app crate is the composition root: it wires the gix-backed project store,
  the dispatch table, the render layer and the resolved settings into one axum
  Router rooted at a project directory. These scenarios boot that assembled
  Router over a gix-built project root and drive it with a single in-process
  request, as a real client would, asserting the wiring end to end — the same
  shape gitweb's CGI request loop served, now as a standalone server.

  Scenario: an empty project root takes gitweb's "No projects found" path
    Given a project root
    When I GET "/"
    Then the response status is 404

  Scenario: the landing page lists a repository
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "alpha.git"

  Scenario: the landing page lists every repository
    Given a project root
    And the root contains repository "alpha.git"
    And the root contains repository "beta.git"
    When I GET "/"
    Then the response status is 200
    And the response body contains "alpha.git"
    And the response body contains "beta.git"

  Scenario: the modernized stylesheet is served as a static asset
    Given a project root
    When I GET "/static/style.css"
    Then the response status is 200
    And the response content type is "text/css; charset=utf-8"

  Scenario: a project action with no handler yet takes the die_error path
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?p=alpha.git&a=summary"
    Then the response status is 400
