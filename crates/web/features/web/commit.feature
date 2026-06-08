Feature: Serving one commit (commit action)
  gitweb's git_commit resolves a commit-ish (HEAD by default), reads the commit,
  and serves a page with its authorship, message, tree and parent links, and the
  changed-files table — an ordinary diff against its parent, or a combined diff
  for a merge. A revision that resolves to nothing is die_error(404).

  Scenario: a commit page shows authorship, message, parents and changed files
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET "/?p=c.git&a=commit"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Ada Lovelace"
    And the response body contains "Rework the engine"
    And the response body contains "Detailed body line."
    And the response body contains "a=commitdiff"
    And the response body contains "parent"
    And the response body contains "README"
    And the response body contains "NEWFILE"
    And the response body contains "new file"
    And the response body contains "deleted"
    And the response body contains "new.txt"
    And the response body contains "moved from"
    And the response body contains "100% similarity"

  Scenario: a commit page links the tree and offers the commitdiff
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET "/?p=c.git&a=commit"
    Then the response status is 200
    And the response body contains "a=tree"
    And the response body contains "(parent: "

  Scenario: a merge commit shows the combined diff and merge navigation
    Given a project root containing a merge repository "m.git"
    And the commit action is served
    When I GET "/?p=m.git&a=commit"
    Then the response status is 200
    And the response body contains "(merge: "
    And the response body contains "file.txt"

  Scenario: a request for a revision that resolves to nothing is not found
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET "/?p=c.git&a=commit&h=0000000000000000000000000000000000000000"
    Then the response status is 404
