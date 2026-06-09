Feature: Ref badges on commit rows
  gitweb decorates a commit in the shortlog, log and history with a small badge
  per ref whose tip is that commit (format_ref_marker / git_get_references). A
  branch head or a lightweight tag points straight at the commit and links to the
  current list view; an annotated tag points at a tag object that peels to the
  commit, so it is "indirect" and links to the tag view instead. The badge text
  drops the namespace prefix; the title keeps it.

  Scenario: A commit no ref points at gets no badges
    Given a ref "refs/heads/next" targeting commit "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there are no markers

  Scenario: A branch head badges the commit at its tip
    Given a ref "refs/heads/next" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there is 1 marker
    And marker 1 has kind "head"
    And marker 1 shows "next"
    And marker 1 is titled "heads/next"
    And marker 1 is not indirect
    And marker 1 links to the "shortlog" action
    And marker 1 targets ref "refs/heads/next"

  Scenario: A lightweight tag links to the current list view
    Given a ref "refs/tags/v1.0" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there is 1 marker
    And marker 1 has kind "tag"
    And marker 1 shows "v1.0"
    And marker 1 is titled "tags/v1.0"
    And marker 1 is not indirect
    And marker 1 links to the "shortlog" action

  Scenario: An annotated tag is indirect and links to the tag view
    Given an annotated tag ref "refs/tags/v2.0" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there is 1 marker
    And marker 1 has kind "tag"
    And marker 1 shows "v2.0"
    And marker 1 is titled "tags/v2.0"
    And marker 1 is indirect
    And marker 1 links to the "tag" action
    And marker 1 targets ref "refs/tags/v2.0"

  Scenario: On the tag page an annotated tag falls back to the shortlog link
    Given an annotated tag ref "refs/tags/v2.0" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "tag" view
    Then there is 1 marker
    And marker 1 links to the "shortlog" action

  Scenario: A direct ref follows the log view
    Given a ref "refs/heads/next" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "log" view
    Then there is 1 marker
    And marker 1 links to the "log" action

  Scenario: A direct ref follows the history view
    Given a ref "refs/heads/next" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "history" view
    Then there is 1 marker
    And marker 1 links to the "history" action

  Scenario: A remote-tracking branch keeps its remote-qualified name
    Given a ref "refs/remotes/origin/next" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there is 1 marker
    And marker 1 has kind "remote"
    And marker 1 shows "origin/next"
    And marker 1 is titled "remotes/origin/next"

  Scenario: An unusual namespace keeps its type as the badge class
    Given a ref "refs/notes/commits" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there is 1 marker
    And marker 1 has kind "note"
    And marker 1 shows "commits"
    And marker 1 is titled "notes/commits"

  Scenario: Several refs at one commit each badge it, refs at other commits are skipped
    Given a ref "refs/heads/next" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    And an annotated tag ref "refs/tags/v2.0" targeting commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    And a ref "refs/heads/other" targeting commit "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    When I compute ref markers for commit "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in the "shortlog" view
    Then there are 2 markers
    And marker 1 shows "next"
    And marker 2 shows "v2.0"
