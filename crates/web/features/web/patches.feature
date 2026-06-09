Feature: Serving a commit range as a numbered format-patch stream (patches action)
  gitweb's patches action streams `git format-patch` for a commit range: the most
  recent commits reachable from the tip (up to the `patches` feature limit),
  oldest-first, each rendered as a `git am`-able mail whose `Subject` is numbered
  `[PATCH i/N]`. It is served as text/plain and offered inline under
  `<project basename>-<hash>.patch`. The view is gated on the `patches` feature —
  off, gitweb answers 403 "Patch view not allowed".

  Scenario: the commit range is served as an inline numbered mailbox stream
    Given a project root containing a commit repository "c.git"
    And the patches action is served
    When I GET "/?p=c.git&a=patches"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response is offered inline as "c.git-HEAD.patch"
    And the response body contains "Subject: [PATCH 1/2] Initial import"
    And the response body contains "Subject: [PATCH 2/2] Rework the engine"
    And the response body contains "Detailed body line."
    And the response body contains "2.54.0"

  Scenario: the filename uses an explicit hash request value
    Given a project root containing a commit repository "c.git"
    And the patches action is served
    When I GET "/?p=c.git&a=patches&h=HEAD"
    Then the response status is 200
    And the response is offered inline as "c.git-HEAD.patch"

  Scenario: the patches view is forbidden when the patches feature is off
    Given a project root containing a commit repository "c.git"
    And the patches action is served with the patches feature off
    When I GET "/?p=c.git&a=patches"
    Then the response status is 403
