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

  Scenario: the composition root serves a project's shortlog
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?p=alpha.git&a=shortlog"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Shortlog"
    And the response body contains "init"

  Scenario: the composition root serves a project's verbose log
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?p=alpha.git&a=log"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Log"
    And the response body contains "init"
    And the response body contains "Ada Lovelace"

  Scenario: the composition root serves a project's summary
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?p=alpha.git&a=summary"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "shortlog"
    And the response body contains "heads"
    And the response body contains "init"

  Scenario: the composition root serves the machine-readable project index
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?a=project_index"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response body contains "alpha.git"

  Scenario: the composition root serves the OPML project outline
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?a=opml"
    Then the response status is 200
    And the response content type is "text/xml; charset=utf-8"
    And the response body contains "<opml version="1.0">"

  Scenario: a project action with no handler yet takes the die_error path
    Given a project root
    And the root contains repository "alpha.git"
    When I GET "/?p=alpha.git&a=commit"
    Then the response status is 400
