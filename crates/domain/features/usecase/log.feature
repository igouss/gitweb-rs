Feature: Log (verbose recent history, a block per commit)
  gitweb's log (git_log -> git_log_generic + git_log_body) is the verbose history:
  unlike the shortlog's one-line table, each commit is a block with a header
  carrying the commit's relative age and subject, an authorship line, and the full
  message body. The use case walks the same windowed history the shortlog does
  (HEAD by default, one commit more than the page to detect a further page) and
  maps each commit into a verbose row: the relative age measured against now, the
  author and the author's absolute date, the subject, and the processed message
  body.

  The header age is always the relative form (gitweb's age_string), even for a
  commit older than two weeks — that is the key difference from the shortlog,
  whose date cell swaps to the absolute date past two weeks.

  An unborn repository (no HEAD) has no history to list and is not an error.

  Scenario: An unborn repository lists no commits and does not error
    Given the current time is 1780842645
    When I assemble the log of the default branch with page size 16
    Then no log entries are listed
    And the log has no further page

  Scenario: A single commit off HEAD becomes one verbose block
    Given the current time is 1780842645
    And the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1700000000 by "Alice" titled "Add the thing"
    When I assemble the log of the default branch with page size 16
    Then the logged commits are "c1"
    And the log has no further page
    And the log entry "c1" is by "Alice"
    And the log entry "c1" shows the title "Add the thing"
    And the log entry "c1" shows the age "2 years ago"
    And the log entry "c1" was authored on "2023-11-14"
    And the log entry "c1" body begins "Add the thing"

  Scenario: A further page is offered only when one more commit exists
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    When I assemble the log of the default branch with page size 2
    Then the logged commits are "c3, c2"
    And the log has a further page

  Scenario: The final page reports no further page
    Given the current time is 1780842645
    And the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1780583445 by "Alice" titled "third"
    And a commit "c2" at epoch 1780583444 by "Bob" titled "second"
    And a commit "c1" at epoch 1780583443 by "Carol" titled "first"
    When I assemble the log of the default branch with page size 5
    Then the logged commits are "c3, c2, c1"
    And the log has no further page

  Scenario: The log of an explicit revision resolves that branch
    Given the current time is 1780842645
    And the repository has branch "topic" committed at 1780583445
    And a commit "c1" at epoch 1780583445 by "Alice" titled "branch tip"
    When I assemble the log of "topic" with page size 16
    Then the logged commits are "c1"
