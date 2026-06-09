Feature: Shortlog (recent history, one line per commit)
  gitweb's shortlog (git_shortlog -> git_log_generic + git_shortlog_body) is the
  recent commit history rendered one line per commit, and the same body the
  summary page embeds. The use case walks the repository's history from a base
  revision (HEAD by default), windowed a page at a time, and turns each commit
  into a row carrying its date cell (the two-week age/date swap), its author name
  chopped for the row, and its subject. To know whether to offer a further page,
  it asks history for one commit more than the page holds and reports the surplus.

  An unborn repository (no HEAD) has no history to list and is not an error.

  Scenario: An unborn repository lists no commits and does not error
    Given the current time is 1780842645
    When I assemble the shortlog of the default branch with page size 16
    Then no commits are listed
    And the shortlog has no further page

  Scenario: A single commit off HEAD becomes one row
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Alice" titled "Add the thing"
    When I assemble the shortlog of the default branch with page size 16
    Then the listed commits are "c1"
    And the shortlog has no further page
    And the commit "c1" is by "Alice"
    And the commit "c1" shows the subject "Add the thing"
    And the commit "c1" date cell shows "3 days ago"

  Scenario: A further page is offered only when one more commit exists
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    When I assemble the shortlog of the default branch with page size 2
    Then the listed commits are "c3, c2"
    And the shortlog has a further page

  Scenario: The final page reports no further page
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    When I assemble the shortlog of the default branch with page size 5
    Then the listed commits are "c3, c2, c1"
    And the shortlog has no further page

  Scenario: The shortlog of an explicit revision resolves that branch
    Given the current time is 1780842645
    And the repository has branch "topic" committed at 1780583445
    And a commit "c1" at epoch 1780583445 by "Alice" titled "branch tip"
    When I assemble the shortlog of "topic" with page size 16
    Then the listed commits are "c1"

  Scenario: A long author name is chopped for the row but kept in full
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Maximilian Schell" titled "x"
    When I assemble the shortlog of the default branch with page size 16
    Then the commit "c1" is by "Maximilian Schell"
    And the commit "c1" author shortens to "Maximilian... "

  Scenario: A commit at a branch tip carries its ref badge
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1780583445 by "Alice" titled "Add the thing"
    And branch "release" points at commit "c1"
    When I assemble the shortlog of the default branch with page size 16
    Then the row for commit "c1" carries the marker "head:release"
