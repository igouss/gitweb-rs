Feature: History (per-path commit log)
  gitweb's history (git_history -> git_log_generic + git_history_body) is the log
  filtered to the commits that touched one file or directory, newest first. The
  use case walks the repository's history from a base revision (HEAD by default)
  limited to a path, windowed a page at a time, and turns each commit into a row
  carrying its date cell (the two-week age/date swap), its author chopped for the
  row, and its subject.

  Two facts are shared by the whole view. The path's type (gitweb's $ftype: a
  blob for a file, a tree for a directory) and its current object (the newest
  version in the view) come from the first commit in the window that still has
  the path. A file row whose blob differs from that current one offers a "diff to
  current"; the newest row, being the current one, does not. A path no commit in
  the window still carries has no resolvable type and is an error, matching
  gitweb's die_error(500, "Unknown type of object").

  Scenario: a path no commit carries cannot be typed and is an error
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Alice" titled "root"
    When I assemble the history of "ghost.txt" from the default branch with page size 16
    Then assembling the history fails

  Scenario: a single commit touching a file becomes one blob row
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Alice" titled "Add the thing"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 16
    Then the history lists "c1"
    And the history file name is "a.txt"
    And the history file type is "blob"
    And the history current blob is "v1"
    And the history has no further page
    And the history row "c1" is by "Alice"
    And the history row "c1" shows the subject "Add the thing"
    And the history row "c1" date cell shows "3 days ago"

  Scenario: a file history lists every touching commit newest first
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And commit "c3" changes file "a.txt" to blob "v3"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And commit "c2" changes file "a.txt" to blob "v2"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 16
    Then the history lists "c3, c2, c1"
    And the history current blob is "v3"

  Scenario: the newest row is the current one and offers no diff to current
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And commit "c3" changes file "a.txt" to blob "v3"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And commit "c2" changes file "a.txt" to blob "v2"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 16
    Then the history row "c3" offers no diff to current
    And the history row "c2" offers a diff to current "v2"
    And the history row "c1" offers a diff to current "v1"

  Scenario: a directory history is typed as a tree and offers no diffs to current
    Given the current time is 1780842645
    And the repository HEAD is at commit "c2"
    And a commit "c2" at epoch 1780583445 by "Alice" titled "second"
    And commit "c2" changes directory "dir" to tree "t2"
    And a commit "c1" at epoch 1780583444 by "Bob" titled "first"
    And commit "c1" changes directory "dir" to tree "t1"
    When I assemble the history of "dir" from the default branch with page size 16
    Then the history lists "c2, c1"
    And the history file type is "tree"
    And the history row "c1" offers no diff to current

  Scenario: when the newest commit deleted the file the current blob is the next one
    Given the current time is 1780842645
    And the repository HEAD is at commit "c2"
    And a commit "c2" at epoch 1780583445 by "Alice" titled "remove a"
    And commit "c2" deletes "a.txt"
    And a commit "c1" at epoch 1780583444 by "Bob" titled "add a"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 16
    Then the history lists "c2, c1"
    And the history file type is "blob"
    And the history current blob is "v1"

  Scenario: a further page is offered only when one more touching commit exists
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And commit "c3" changes file "a.txt" to blob "v3"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And commit "c2" changes file "a.txt" to blob "v2"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 2
    Then the history lists "c3, c2"
    And the history has a further page

  Scenario: a long author name is chopped for the row but kept in full
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Wolfgang Amadeus Mozart" titled "x"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from the default branch with page size 16
    Then the history row "c1" is by "Wolfgang Amadeus Mozart"
    And the history row "c1" author shortens to "Wolfgang Amadeus... "

  Scenario: the history of an explicit revision resolves that branch
    Given the current time is 1780842645
    And the repository has branch "topic" committed at 1780583445
    And a commit "c1" at epoch 1780583445 by "Alice" titled "branch tip"
    And commit "c1" changes file "a.txt" to blob "v1"
    When I assemble the history of "a.txt" from "topic" with page size 16
    Then the history lists "c1"
