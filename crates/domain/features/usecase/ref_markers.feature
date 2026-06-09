Feature: Ref markers indexed for a commit-list body (ref-markers use case)
  gitweb reads every reference once (git_get_references) before walking the log,
  then badges each row with the refs whose tip is that commit (format_ref_marker).
  The use case is that single read, indexed for one view: a branch head or a
  lightweight tag points straight at its commit; an annotated tag is peeled to its
  commit and reported indirect (shown here with a trailing "*"); refs at other
  commits never appear on a row.

  Scenario: A commit no ref points at carries no markers
    Given branch "main" points at commit "c1"
    When I index ref markers for the "shortlog" view
    Then commit "c2" has no markers

  Scenario: Heads, lightweight and annotated tags all badge their commit, in ref-name order
    Given branch "main" points at commit "c1"
    And a lightweight tag "v1.0" points at commit "c1"
    And an annotated tag "v2.0" points at commit "c1"
    When I index ref markers for the "shortlog" view
    Then the markers for commit "c1" are "head:main, tag:v1.0, tag*:v2.0"

  Scenario: Only the refs whose tip is the commit appear on that row
    Given branch "main" points at commit "c1"
    And branch "topic" points at commit "c2"
    When I index ref markers for the "shortlog" view
    Then the markers for commit "c1" are "head:main"
    And the markers for commit "c2" are "head:topic"
