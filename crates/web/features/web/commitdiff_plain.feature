Feature: Serving the raw commitdiff (commitdiff_plain action)
  gitweb's commitdiff_plain frames a commit's raw diff as a `git am`-able patch:
  a mailbox header (From / Date / Subject / X-Git-Url), the comment, a `---`
  separator, then the patch. It is served as text/plain and offered inline under
  `<project basename>-<hash>.patch`. The X-Git-Url line carries the request's own
  absolute self URL, and gitweb stamps the filename with the request hash value
  (the literal HEAD when none is given), not the resolved id.

  Scenario: the raw commitdiff is a mailbox-framed patch served inline
    Given a project root containing a commit repository "c.git"
    And the commitdiff_plain action is served
    When I GET "/?p=c.git&a=commitdiff_plain"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response is offered inline as "c.git-HEAD.patch"
    And the response body contains "From: Ada Lovelace"
    And the response body contains "Subject: Rework the engine"
    And the response body contains "Detailed body line."
    And the response body contains "X-Git-Url: http://localhost?p=c.git;a=commitdiff_plain"
    And the response body contains "diff --git a/README b/README"

  Scenario: the self URL echoes an explicit hash, and the filename uses it
    Given a project root containing a commit repository "c.git"
    And the commitdiff_plain action is served
    When I GET "/?p=c.git&a=commitdiff_plain&h=HEAD"
    Then the response status is 200
    And the response is offered inline as "c.git-HEAD.patch"
    And the response body contains "X-Git-Url: http://localhost?p=c.git;a=commitdiff_plain;h=HEAD"

  Scenario: a commit with a parent has no bare commit-id line before the diff
    Given a project root containing a commit repository "c.git"
    And the commitdiff_plain action is served
    When I GET "/?p=c.git&a=commitdiff_plain"
    Then the response status is 200
    And the response body contains "---"
    And the response body contains "diff --git a/README b/README"

  Scenario: a raw commitdiff for a revision that resolves to nothing is not found
    Given a project root containing a commit repository "c.git"
    And the commitdiff_plain action is served
    When I GET "/?p=c.git&a=commitdiff_plain&h=0000000000000000000000000000000000000000"
    Then the response status is 404
