Feature: Serving a file's history (history action)
  gitweb's git_history opens the project, walks the commits that touched a given
  path from HEAD (or a named revision), and serves them as one row per commit:
  the commit's date, its author, its subject linking to the commit, and a links
  cell offering the file's blob/tree, commitdiff, raw, and — for an older version
  — a "diff to current". A request for a project that does not exist is
  die_error(404); a request with no file is die_error(400); a path no commit ever
  carried cannot be typed and is die_error(500).

  Scenario: the history page lists a file's commits with per-commit links
    Given a repository "fh.git" with a file history
    And the history action is served
    When I GET "/?p=fh.git&a=history&f=file.txt"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "History"
    And the response body contains "file.txt"
    And the response body contains "edit file.txt"
    And the response body contains "add file.txt"
    And the response body contains "Ada Lovelace"
    And the response body contains "commitdiff"
    And the response body contains "diff to current"
    And the response body contains "2023-11-15"
    And the response body contains "2023-11-14"

  Scenario: history for a project that does not exist is not found
    Given a repository "fh.git" with a file history
    And the history action is served
    When I GET "/?p=ghost.git&a=history&f=file.txt"
    Then the response status is 404

  Scenario: history with no file is a bad request
    Given a repository "fh.git" with a file history
    And the history action is served
    When I GET "/?p=fh.git&a=history"
    Then the response status is 400

  Scenario: history of a path no commit ever carried cannot be typed
    Given a repository "fh.git" with a file history
    And the history action is served
    When I GET "/?p=fh.git&a=history&f=ghost.txt"
    Then the response status is 500

  Scenario: the commit at the branch tip is badged with its ref
    Given a repository "fh.git" with a file history
    And the history action is served
    When I GET "/?p=fh.git&a=history&f=file.txt"
    Then the response status is 200
    And the response body contains "<span class="refs">"
    And the response body contains "class="ref head""
    And the response body contains ">main</a>"
