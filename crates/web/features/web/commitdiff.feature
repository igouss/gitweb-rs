Feature: Serving a commit's diff (commitdiff action + clean-diff endpoint)
  gitweb's git_commitdiff colourises a commit's diff in the page. We modernize
  that: the commitdiff action serves a host page — the commit's authorship and
  message, the changed-files table (with patch-anchor links), the raw/patch
  format affordances and the parent/merge context — whose #diff-root the client
  viewer fills from a clean unified diff it fetches from a separate endpoint. The
  clean-diff endpoint serves the bare bytes git emits, starting at "diff --git",
  as text/plain — not commitdiff_plain, whose mail header would break the viewer.

  Scenario: the commitdiff host page shows authorship, message, files and the diff root
    Given a project root containing a commit repository "c.git"
    And the commitdiff action is served
    When I GET "/?p=c.git&a=commitdiff"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Ada Lovelace"
    And the response body contains "Rework the engine"
    And the response body contains "Detailed body line."
    And the response body contains "README"
    And the response body contains "id="diff-root""
    And the response body contains "data-diff-url="/diff?p=c.git"
    And the response body contains "diff-viewer.js"

  Scenario: the commitdiff host page offers the raw and patch formats and the parent context
    Given a project root containing a commit repository "c.git"
    And the commitdiff action is served
    When I GET "/?p=c.git&a=commitdiff"
    Then the response status is 200
    And the response body contains "a=commitdiff_plain"
    And the response body contains "a=patch"
    And the response body contains "(parent: "
    And the response body contains "href="#patch1""

  Scenario: a merge commitdiff shows the combined files and merge navigation, no patch
    Given a project root containing a merge repository "m.git"
    And the commitdiff action is served
    When I GET "/?p=m.git&a=commitdiff"
    Then the response status is 200
    And the response body contains "(merge: "
    And the response body contains "file.txt"
    And the response body contains "diff2"
    And the response body does not contain "a=patch"

  Scenario: a merge commitdiff against an explicit non-first parent shows that parent's two-tree diff
    Given a project root containing a merge repository "m.git"
    And the commitdiff action is served
    When I GET the commitdiff of "m.git" against its second parent
    Then the response status is 200
    And the response body contains "(from parent 2"
    And the response body contains "file.txt"
    And the response body contains "hp="
    And the response body does not contain "diff2"
    And the response body does not contain "(merge: "

  Scenario: a commitdiff for a revision that resolves to nothing is not found
    Given a project root containing a commit repository "c.git"
    And the commitdiff action is served
    When I GET "/?p=c.git&a=commitdiff&h=0000000000000000000000000000000000000000"
    Then the response status is 404

  Scenario: the clean-diff endpoint serves the bare unified diff as text/plain
    Given a project root containing a commit repository "c.git"
    When I GET "/diff?p=c.git&h=HEAD"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response body contains "diff --git a/README b/README"
    And the response body does not contain "From: "

  Scenario: the clean-diff endpoint defaults the commit to HEAD
    Given a project root containing a commit repository "c.git"
    When I GET "/diff?p=c.git"
    Then the response status is 200
    And the response body contains "diff --git"

  Scenario: the clean-diff endpoint rejects an unsafe ref
    Given a project root containing a commit repository "c.git"
    When I GET "/diff?p=c.git&h=../etc/passwd"
    Then the response status is 400
