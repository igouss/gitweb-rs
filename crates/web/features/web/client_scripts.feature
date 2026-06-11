Feature: Serving the gitweb client-side script assets
  gitweb ships three feature-specific client behaviours: timezone-adjusting dates
  (javascript-timezone), JavaScript-driven action links (javascript-actions), and
  progressively filled blame (blame_incremental). Each is a modern, framework-free
  ES module baked into the binary and served at a stable URL with a JavaScript
  MIME type, the way the stylesheet, favicon and diff-viewer module are — no
  filesystem, nothing to go missing at runtime. The in-browser effect of each
  module (timezone conversion, link rewiring, progressive blame) is a documented
  manual check, not a headless assertion; this feature pins only that the assets
  are served correctly and carry their load-bearing logic.

  Scenario: the timezone module is served as an ES module with a JavaScript MIME type
    Given an empty project root
    When I GET "/static/gitweb-timezone.js"
    Then the response status is 200
    And the response content type is "text/javascript; charset=utf-8"

  Scenario: the timezone module rewrites the datetime spans and persists the choice
    Given an empty project root
    When I GET "/static/gitweb-timezone.js"
    Then the response body contains "datetime"
    And the response body contains "gitweb_tz"

  Scenario: the actions module is served as an ES module with a JavaScript MIME type
    Given an empty project root
    When I GET "/static/gitweb-actions.js"
    Then the response status is 200
    And the response content type is "text/javascript; charset=utf-8"

  Scenario: the actions module adds the js=1 parameter to links
    Given an empty project root
    When I GET "/static/gitweb-actions.js"
    Then the response body contains "js=1"

  Scenario: the incremental-blame module is served as an ES module with a JavaScript MIME type
    Given an empty project root
    When I GET "/static/blame-incremental.js"
    Then the response status is 200
    And the response content type is "text/javascript; charset=utf-8"

  Scenario: the incremental-blame module exports the startBlame entry point
    Given an empty project root
    When I GET "/static/blame-incremental.js"
    Then the response body contains "startBlame"
    And the response body contains "blame_data"

  # ---- feature-gated <script> wiring in the document head -------------------
  # gitweb wires each feature's <script> only when that feature is on. The
  # javascript-timezone feature ships ON by default (the 'local' default); the
  # javascript-actions feature ships OFF by default. A normally-served HTML page
  # therefore carries the timezone module and not the actions module.

  Scenario: a served page wires the timezone module when javascript-timezone is on
    Given a project root containing repository "tz.git"
    And the summary action is served
    When I GET "/?p=tz.git&a=summary"
    Then the response status is 200
    And the response body contains "<script type="module" src="/static/gitweb-timezone.js" defer></script>"

  Scenario: a served page omits the actions module when javascript-actions is off
    Given a project root containing repository "tz.git"
    And the summary action is served
    When I GET "/?p=tz.git&a=summary"
    Then the response status is 200
    And the response body does not contain "gitweb-actions.js"

  Scenario: a served page wires the actions module when javascript-actions is on
    Given a project root containing repository "act.git"
    And the summary action is served with javascript-actions on
    When I GET "/?p=act.git&a=summary"
    Then the response status is 200
    And the response body contains "<script type="module" src="/static/gitweb-actions.js" defer></script>"
