Feature: Rendering the commit view
  The commit view (modernized git_commit) shows the parent/merge context in the
  navigation, an object-header table (the author and committer authorship rows,
  the commit id, the tree link, and one row per parent), the message body, and
  the changed-files table. URLs are built by the web boundary, so this layer
  takes finished hrefs and decides only layout, escaping, the absolute authorship
  dates, and the per-file status notes and links.

  Scenario: a single-parent commit renders its metadata, message and changed files
    When I render a single-parent commit page
    Then the result contains "Ada Lovelace"
    And the result contains "ada@example.com"
    And the result contains "Linus Torvalds"
    And the result contains "(parent: "
    And the result contains "/r/commit/parent01"
    And the result contains "<a href="/r/commitdiff/parent01">diff</a>"
    And the result contains "/r/tree/c0ffee"
    And the result contains ">tree<"
    And the result contains "Add the analytical engine"
    And the result contains "3 files changed"
    And the result contains "engine.rs"
    And the result contains "[changed mode: 0644"
    And the result contains "0755]"
    And the result contains "[new file with mode: 0644]"
    And the result contains "new.rs"
    And the result contains "moved from"
    And the result contains "/r/blob/old"
    And the result contains "95% similarity"

  Scenario: a single-parent commit links the author date as a zone-aware time
    When I render a single-parent commit page
    Then the result contains "<time"
    And the result contains "datetime="

  Scenario: a root commit shows the initial marker and no parent rows
    When I render a root commit page
    Then the result contains "(initial)"
    And the result does not contain "(parent:"
    And the result does not contain "(merge:"

  Scenario: a merge commit lists every parent in the navigation
    When I render a merge commit page
    Then the result contains "(merge: "
    And the result contains "/r/commit/par1"
    And the result contains "/r/commit/par2"
