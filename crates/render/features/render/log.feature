Feature: Rendering the verbose log
  The verbose log (modernized git_log_body) shows each commit as a block, not a
  table row: a header carrying the commit's relative age and subject (linking to
  the commit), the per-commit commit / commitdiff / tree links, an authorship line
  with the author and the absolute date the commit was authored, and the message
  body with one entry per processed line. A sign-off line is styled apart, and an
  xxxlink trailer becomes a link. A trailing "more" link appears only when a
  further page exists. URLs are built by the web boundary, so this layer takes
  finished hrefs and only decides layout, escaping, and the timestamp markup.

  Scenario: a commit block shows its age, subject link, authorship and links
    Given a log entry by "Alice" titled "Add the thing" aged "3 days ago" at "/r/commit/abc"
    When I render the log entries
    Then the result contains "3 days ago"
    And the result contains "Add the thing"
    And the result contains "/r/commit/abc"
    And the result contains "/r/commit/abc/diff"
    And the result contains "/r/commit/abc/tree"
    And the result contains "Alice"
    And the result contains "commitdiff"

  Scenario: the authorship line carries the absolute timestamp badge
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    When I render the log entries
    Then the result contains "Sun, 7 Jun 2026 14:30:45 +0000"
    And the result contains ">16:30<"

  Scenario: a sign-off line in the body is styled apart
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    And its body is the sign-off "Signed-off-by: Ada Lovelace"
    When I render the log entries
    Then the result contains "class="signoff""
    And the result contains "Signed-off-by: Ada Lovelace"

  Scenario: an autolink line in the body links its URL
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    And its body links "Link" to "https://example.com/issue/1"
    When I render the log entries
    Then the result contains "https://example.com/issue/1"
    And the result contains "<a"

  Scenario: the more link appears when a further page is offered
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    And the log offers more at "/r/log?pg=1" labelled "next"
    When I render the log entries
    Then the result contains "/r/log?pg=1"
    And the result contains "next"

  Scenario: no more link appears when there is no further page
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    When I render the log entries
    Then the result does not contain "pg=1"

  Scenario: a commit at a ref tip shows the ref badge after its subject
    Given a log entry badged a "tag indirect" ref "v2.0" titled "tags/v2.0" linking "/r/tag/v2.0"
    When I render the log entries
    Then the result contains "<span class="refs">"
    And the result contains "class="ref tag indirect""
    And the result contains "title="tags/v2.0""
    And the result contains "/r/tag/v2.0"
    And the result contains ">v2.0</a>"

  Scenario: a commit with no ref tip has no refs span
    Given a log entry by "Alice" titled "x" aged "now" at "/c"
    When I render the log entries
    Then the result does not contain "class="refs""
