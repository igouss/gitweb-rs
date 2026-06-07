Feature: Error responses (die_error)
  gitweb's die_error turns a failure into a styled page served with the right
  HTTP status. This cross-cutting concern is two things: classifying a domain
  failure into a status, and rendering a "status - message" page (with an
  optional detail block). The page is modernized to a semantic <main>; the
  message is escaped, the detail is already-safe markup. Verified behaviourally,
  since the markup is modernized rather than byte-identical to gitweb's.

  # ---- classifying a domain failure into a status --------------------------

  Scenario: a missing object, ref or project is 404 Not Found
    Given a not-found failure "No such project"
    When I render the error response
    Then the HTTP status is 404
    And the status title is "404 Not Found"
    And the body contains "404 - "
    And the body contains "No such project"

  Scenario: malformed input is 400 Bad Request
    Given an invalid-input failure "Invalid hash parameter"
    When I render the error response
    Then the HTTP status is 400
    And the status title is "400 Bad Request"
    And the body contains "400 - "

  Scenario: a policy denial is 403 Forbidden
    Given a forbidden failure "Blame view not allowed"
    When I render the error response
    Then the HTTP status is 403
    And the status title is "403 Forbidden"
    And the body contains "403 - "
    And the body contains "Blame view not allowed"

  Scenario: an internal backend failure is 500 Internal Server Error
    Given a backend failure "Open git-ls-tree failed"
    When I render the error response
    Then the HTTP status is 500
    And the status title is "500 Internal Server Error"
    And the body contains "500 - "

  # ---- request-level rejections rendered directly --------------------------

  Scenario: a service-unavailable page is rendered without a domain failure
    When I render a 503 error page saying "The load average on the server is too high"
    Then the HTTP status is 503
    And the status title is "503 Service Unavailable"
    And the body contains "503 - The load average on the server is too high"

  # ---- the page shape ------------------------------------------------------

  Scenario: the error page is a semantic main element
    When I render a 404 error page saying "No such project"
    Then the body contains "<main class="error">"
    And the body contains "<p class="error-status">404 - No such project</p>"

  Scenario: an error without detail has no detail block
    When I render a 404 error page saying "No such project"
    Then the body does not contain "<hr>"
    And the body does not contain "error-detail"

  Scenario: an error with detail shows it below a rule
    When I render a 400 error page saying "Invalid search regexp" with detail "<pre>nothing to repeat</pre>"
    Then the body contains "<hr>"
    And the body contains "<div class="error-detail"><pre>nothing to repeat</pre></div>"

  # ---- escaping ------------------------------------------------------------

  Scenario: the message is escaped, not interpreted as markup
    When I render a 500 error page saying "Tom & Jerry <broke> it"
    Then the body contains "Tom &amp; Jerry &lt;broke&gt; it"
    And the body does not contain "<broke>"
