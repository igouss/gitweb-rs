Feature: Rendering the shortlog table
  The shortlog table (modernized git_shortlog_body) shows one row per commit: a
  date cell holding the shown text with the alternate form as its hover tooltip,
  the author name (chopped for the row, with the full name as a tooltip when it
  was chopped), the commit subject linking to the commit (the short subject with
  the full one as a tooltip when chopped), and the per-commit commit / commitdiff
  / tree links. A trailing "more" row appears only when a further page exists.
  URLs are built by the web boundary, so this layer takes finished hrefs and only
  decides layout and escaping.

  Scenario: a commit row shows its date, author, subject link, and per-commit links
    Given a shortlog commit by "Alice" titled "Add the thing" dated "3 days ago" at "/r/commit/abc"
    When I render the shortlog table
    Then the result contains "3 days ago"
    And the result contains "Add the thing"
    And the result contains "/r/commit/abc"
    And the result contains "/r/commit/abc/diff"
    And the result contains "/r/commit/abc/tree"

  Scenario: the date cell carries the alternate form as a tooltip
    Given a shortlog commit by "Alice" titled "x" dated "3 days ago" tooltip "2026-06-04" at "/c"
    When I render the shortlog table
    Then the result contains "title="2026-06-04""
    And the result contains "<i>3 days ago</i>"

  Scenario: a chopped author shows the full name as a tooltip
    Given a shortlog commit authored "Maximilian Schell" shortened to "Maximilian... " at "/c"
    When I render the shortlog table
    Then the result contains "Maximilian... "
    And the result contains "title="Maximilian Schell""

  Scenario: an unchopped author is shown plainly without a name tooltip
    Given a shortlog commit by "Alice" titled "x" dated "now" at "/c"
    When I render the shortlog table
    Then the result contains "Alice"
    And the result does not contain "title="Alice""

  Scenario: a chopped subject links the short form with the full subject as a tooltip
    Given a shortlog commit subject "A very long commit subject that exceeds the limit and is chopped here" shortened to "A very long commit subject that is chopped" at "/c"
    When I render the shortlog table
    Then the result contains "A very long commit subject that is chopped"
    And the result contains "title="A very long commit subject that exceeds the limit and is chopped here""

  Scenario: the more row appears when a further page is offered
    Given a shortlog commit by "Alice" titled "x" dated "now" at "/c"
    And the shortlog offers more at "/r/shortlog?pg=1" labelled "next"
    When I render the shortlog table
    Then the result contains "/r/shortlog?pg=1"
    And the result contains "next"

  Scenario: no more row appears when there is no further page
    Given a shortlog commit by "Alice" titled "x" dated "now" at "/c"
    When I render the shortlog table
    Then the result does not contain "pg=1"

  Scenario: a commit at a ref tip shows the ref badge after its subject
    Given a shortlog commit badged a "tag indirect" ref "v2.0" titled "tags/v2.0" linking "/r/tag/v2.0"
    When I render the shortlog table
    Then the result contains "<span class="refs">"
    And the result contains "class="ref tag indirect""
    And the result contains "title="tags/v2.0""
    And the result contains "/r/tag/v2.0"
    And the result contains ">v2.0</a>"

  Scenario: a commit with no ref tip has no refs span
    Given a shortlog commit by "Alice" titled "x" dated "now" at "/c"
    When I render the shortlog table
    Then the result does not contain "class="refs""
