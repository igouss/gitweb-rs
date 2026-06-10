Feature: The changed-files table base
  gitweb's git_commitdiff builds the changed-files TABLE (git_difftree_body) from
  a diff it picks by $hash_parent_param / $use_parents: an explicit parent makes
  it a two-tree diff against that parent (git diff-tree -r $hash_parent $hash,
  $use_parents false), a merge with no explicit parent is the COMBINED diff
  against every parent (gitweb's --cc), a single-parent commit is the two-tree
  diff against its parent, and a root commit is the two-tree diff against the
  empty tree (gitweb's --root).

  This differs from the viewer's diff base (the commitdiff.feature rule), which
  has no combined form and reduces a merge to its first parent: the TABLE shows
  the real combined diff for a merge, but an explicit parent always wins and
  makes it an ordinary two-tree diff against that parent — which is why a merge
  viewed against an explicit non-first parent must show that parent's diff, not
  the combined one. Picking the shape and base is the pure rule here.

  Scenario: A root commit's table is an ordinary diff against the empty tree
    Given a commit with no parents
    When I pick the changed-files base
    Then the changed-files table is ordinary against the empty tree

  Scenario: A single-parent commit's table is an ordinary diff against its parent
    Given a commit with parent "aaaa"
    When I pick the changed-files base
    Then the changed-files table is ordinary against commit "aaaa"

  Scenario: A merge's table is the combined diff against every parent
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    When I pick the changed-files base
    Then the changed-files table is combined

  Scenario: An octopus merge's table is still combined
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    And the commit also has parent "cccc"
    When I pick the changed-files base
    Then the changed-files table is combined

  Scenario: An explicit first parent makes a merge's table ordinary against it
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    And an explicit parent "aaaa" is given
    When I pick the changed-files base
    Then the changed-files table is ordinary against commit "aaaa"

  Scenario: An explicit non-first parent makes a merge's table ordinary against it
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    And an explicit parent "bbbb" is given
    When I pick the changed-files base
    Then the changed-files table is ordinary against commit "bbbb"

  Scenario: An explicit parent wins even for a single-parent commit
    Given a commit with parent "aaaa"
    And an explicit parent "cccc" is given
    When I pick the changed-files base
    Then the changed-files table is ordinary against commit "cccc"
