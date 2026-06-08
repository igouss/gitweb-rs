Feature: The commitdiff diff base
  gitweb's git_commitdiff diffs a commit against a base tree it picks from the
  commit's parents: an explicit parent when one is given, otherwise the single
  parent, the first parent of a merge, or — for a root commit — the empty tree
  (gitweb's "$hash_parent_param = @parents > 1 ? '--cc' : $parent || '--root'",
  reduced to the two-tree forms our diff viewer renders, so a merge with no
  explicit parent diffs against its first parent rather than a combined diff).

  This is the pure rule shared by the commitdiff host page (which builds the
  viewer's diff URL from the base) and the clean-diff endpoint (which feeds the
  base to the patch port). Picking the base is one thing; reading the patch is
  another — only the first lives here.

  Scenario: A root commit diffs against the empty tree
    Given a commit with no parents
    When I pick the commitdiff base
    Then the commitdiff base is the empty tree

  Scenario: A single-parent commit diffs against its parent
    Given a commit with parent "aaaa"
    When I pick the commitdiff base
    Then the commitdiff base is commit "aaaa"

  Scenario: A merge diffs against its first parent
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    When I pick the commitdiff base
    Then the commitdiff base is commit "aaaa"

  Scenario: An octopus merge still diffs against its first parent
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    And the commit also has parent "cccc"
    When I pick the commitdiff base
    Then the commitdiff base is commit "aaaa"

  Scenario: An explicit parent overrides the first parent
    Given a commit with parent "aaaa"
    And the commit also has parent "bbbb"
    And an explicit parent "bbbb" is given
    When I pick the commitdiff base
    Then the commitdiff base is commit "bbbb"

  Scenario: An explicit parent overrides even a root commit
    Given a commit with no parents
    And an explicit parent "aaaa" is given
    When I pick the commitdiff base
    Then the commitdiff base is commit "aaaa"
