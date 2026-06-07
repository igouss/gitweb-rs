Feature: Dispatching a request and serving the response
  The web boundary is gitweb's CGI entry point as a standalone axum server. Once
  a URL is resolved into a validated request, the dispatcher routes it (gitweb's
  dispatch defaulting + project gate) to the registered handler and serves the
  view; a domain failure becomes gitweb's die_error page under the right status.
  Static assets are served straight from the render layer with the correct MIME.

  Each scenario drives the assembled router with one in-process request over a
  gix-built project root, wiring stub handlers as the capability beads will.

  # --- a known action reaches its handler ---

  Scenario: A known action dispatches to its handler
    Given a project root containing repository "solo.git"
    And a stub "summary" page handler
    When I GET "/solo.git/summary"
    Then the response status is 200
    And the response body contains "STUB:summary"

  # --- gitweb's dispatch defaulting: project-list vs summary ---

  Scenario: A bare request lists the projects
    Given an empty project root
    And a stub "project_list" page handler
    When I GET "/"
    Then the response status is 200
    And the response body contains "STUB:project_list"

  Scenario: A project with no action shows its summary
    Given a project root containing repository "solo.git"
    And a stub "summary" page handler
    When I GET "/solo.git"
    Then the response status is 200
    And the response body contains "STUB:summary"

  # --- an action with no handler takes gitweb's Unknown action path ---

  Scenario: An unserved action is the unknown-action error
    Given a project root containing repository "solo.git"
    And a stub "summary" page handler
    When I GET "/solo.git/log"
    Then the response status is 400
    And the response body contains "Unknown action"

  # --- response content types: an HTML page and a plain-text endpoint ---

  Scenario: An HTML page is served as text/html
    Given a project root containing repository "solo.git"
    And a stub "summary" page handler
    When I GET "/solo.git/summary"
    Then the response content type is "text/html; charset=utf-8"

  Scenario: A plain-text endpoint is served as text/plain
    Given a project root containing repository "solo.git"
    And a stub "commitdiff_plain" plain-text handler
    When I GET "/solo.git/commitdiff_plain"
    Then the response content type is "text/plain; charset=utf-8"

  # --- static assets carry the correct MIME type ---

  Scenario: The stylesheet is served as text/css
    Given an empty project root
    When I GET "/static/style.css"
    Then the response status is 200
    And the response content type is "text/css; charset=utf-8"

  Scenario: The favicon is served as image/svg+xml
    Given an empty project root
    When I GET "/static/favicon.svg"
    Then the response status is 200
    And the response content type is "image/svg+xml"

  # --- a domain failure maps to gitweb's die_error status ---

  Scenario: An unsafe project request is the not-found error
    Given a project root containing repository "solo.git"
    When I GET "/?p=../etc"
    Then the response status is 404
