Feature: Serving a single file's diff (blobdiff action + scoped clean diff)
  gitweb's git_blobdiff colourises one file's diff between two base commits in
  the page. We modernize that the way commitdiff does: the blobdiff action serves
  a host page — the base commit's subject, the file path, and the raw affordance —
  whose #diff-root the client viewer fills from a clean unified diff it fetches
  from the diff-text endpoint, scoped to the one file (the diff URL carries f).
  The raw affordance points at blobdiff_plain. A path the diff does not touch is
  404; a missing base is 404.

  Scenario: the blobdiff host page shows the file, the subject and the scoped diff root
    Given a project root containing a commit repository "c.git"
    And the blobdiff action is served
    When I GET the blobdiff of "README" in "c.git"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "README"
    And the response body contains "Rework the engine"
    And the response body contains "id="diff-root""
    And the response body contains "data-diff-url="/diff?"
    And the response body contains "f=README"
    And the response body contains "diff-viewer.js"
    And the response body contains "a=blobdiff_plain"

  Scenario: a renamed file shows its new path and carries the old path in the raw link
    Given a project root containing a commit repository "c.git"
    And the blobdiff action is served
    When I GET the blobdiff of "new.txt" in "c.git"
    Then the response status is 200
    And the response body contains "new.txt"
    And the response body contains "f=new.txt"
    And the response body contains "fp=old.txt"

  Scenario: a blobdiff for a path the diff does not touch is not found
    Given a project root containing a commit repository "c.git"
    And the blobdiff action is served
    When I GET the blobdiff of "absent.txt" in "c.git"
    Then the response status is 404

  Scenario: a blobdiff with a missing base is not found
    Given a project root containing a commit repository "c.git"
    And the blobdiff action is served
    When I GET "/?p=c.git&a=blobdiff&f=README"
    Then the response status is 404

  Scenario: the scoped clean-diff endpoint serves just the one file's diff
    Given a project root containing a commit repository "c.git"
    When I GET the file diff of "README" in "c.git"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response body contains "diff --git a/README b/README"
    And the response body does not contain "NEWFILE"

  Scenario: the scoped clean-diff endpoint 404s a path the diff does not touch
    Given a project root containing a commit repository "c.git"
    When I GET the file diff of "absent.txt" in "c.git"
    Then the response status is 404
