Feature: Rendering the per-path history table
  The history table (modernized git_history_body) shows one row per commit that
  touched a file or directory: a date cell holding the shown text with the
  alternate as its hover tooltip, the author (chopped for the row, full name as a
  tooltip when chopped), the commit subject linking to the commit, and a links
  cell. The links cell is what sets history apart from the shortlog: the file's
  blob/tree at that commit (labelled with the file's type), commitdiff, and — for
  a file — raw plus, when this commit's content differs from the current version,
  "diff to current". A directory has no raw and no diff to current. A trailing
  "more" row appears only when a further page exists. URLs are built by the web
  boundary, so this layer takes finished hrefs.

  Scenario: a blob row shows its date, author, subject link, and per-commit links
    Given a history blob row by "Alice" titled "Add the thing" dated "3 days ago" at "/r/commit/abc"
    When I render the history table
    Then the result contains "3 days ago"
    And the result contains "Add the thing"
    And the result contains "/r/commit/abc"
    And the result contains "/r/commit/abc/blob"
    And the result contains "/r/commit/abc/diff"
    And the result contains "/r/commit/abc/raw"
    And the result contains ">blob<"
    And the result contains "commitdiff"

  Scenario: a blob row offering a diff to current shows the blobdiff link
    Given a history blob row by "Alice" titled "x" dated "now" at "/c"
    And the history row offers a diff to current at "/c/blobdiff"
    When I render the history table
    Then the result contains "/c/blobdiff"
    And the result contains "diff to current"

  Scenario: a blob row not offering a diff to current omits the blobdiff link
    Given a history blob row by "Alice" titled "x" dated "now" at "/c"
    When I render the history table
    Then the result does not contain "diff to current"

  Scenario: a tree row labels the ftype link tree and shows no raw or diff to current
    Given a history tree row by "Alice" titled "x" dated "now" at "/d"
    When I render the history table
    Then the result contains "/d/tree"
    And the result contains ">tree<"
    And the result does not contain "/d/raw"
    And the result does not contain "diff to current"

  Scenario: the date cell carries the alternate form as a tooltip
    Given a history blob row by "Alice" titled "x" dated "3 days ago" tooltip "2026-06-04" at "/c"
    When I render the history table
    Then the result contains "title="2026-06-04""
    And the result contains "<i>3 days ago</i>"

  Scenario: a chopped author shows the full name as a tooltip
    Given a history blob row authored "Wolfgang Amadeus Mozart" shortened to "Wolfgang Amadeus... " at "/c"
    When I render the history table
    Then the result contains "Wolfgang Amadeus... "
    And the result contains "title="Wolfgang Amadeus Mozart""

  Scenario: a chopped subject links the short form with the full subject as a tooltip
    Given a history blob row subject "A very long commit subject that exceeds the limit and is chopped here" shortened to "A very long commit subject that is chopped" at "/c"
    When I render the history table
    Then the result contains "A very long commit subject that is chopped"
    And the result contains "title="A very long commit subject that exceeds the limit and is chopped here""

  Scenario: the more row appears when a further page is offered
    Given a history blob row by "Alice" titled "x" dated "now" at "/c"
    And the history offers more at "/r/history?pg=1" labelled "next"
    When I render the history table
    Then the result contains "/r/history?pg=1"
    And the result contains "next"

  Scenario: no more row appears when there is no further page
    Given a history blob row by "Alice" titled "x" dated "now" at "/c"
    When I render the history table
    Then the result does not contain "pg=1"

  Scenario: a commit at a ref tip shows the ref badge after its subject
    Given a history blob row badged a "tag indirect" ref "v2.0" titled "tags/v2.0" linking "/r/tag/v2.0"
    When I render the history table
    Then the result contains "<span class="refs">"
    And the result contains "class="ref tag indirect""
    And the result contains "title="tags/v2.0""
    And the result contains "/r/tag/v2.0"
    And the result contains ">v2.0</a>"

  Scenario: a commit with no ref tip has no refs span
    Given a history blob row by "Alice" titled "x" dated "now" at "/c"
    When I render the history table
    Then the result does not contain "class="refs""
