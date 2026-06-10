Feature: name-rev --tags ancestor naming (git_get_rev_name_tags)
  gitweb's `git_get_rev_name_tags` runs `git name-rev --tags <oid>` to stamp the
  `X-Git-Tag` line on `commitdiff_plain` / `patch`. name-rev names a commit after
  the nearest tag that reaches it, suffixing the path it took: each first-parent
  generation is `~1`, and stepping onto the Nth parent (N>1) of a merge is `^N`.
  The tag whose tip IS the commit names it at distance zero — bare for a
  lightweight tag, `^0` for an annotated one (name-rev's one-dereference marker),
  and that `^0` is stripped before any `~N` is appended to an ancestor.

  This is the pure naming algorithm (git's `name_rev` propagation + `is_better_name`
  tie-break), ported byte-for-byte from builtin/name-rev.c. The adapter resolves
  each tag to its peeled commit, name, tagger date (the commit's own date for a
  lightweight tag) and dereference flag, and hands this rule the commit ancestry;
  every expected string below was confirmed against real `git name-rev --tags`.

  # --- distance: first-parent generations are ~N ---

  Scenario: a commit several first-parent generations behind a tag is named with ~N
    Given a root commit "n0"
    And a commit "n1" with parent "n0"
    And a commit "n2" with parent "n1"
    And a commit "n3" with parent "n2"
    And an annotated tag "rel" at "n3" with tagger date 100
    When I name "n0" by tags
    Then the rev-name is "rel~3"

  # --- distance zero: the tag's own tip ---

  Scenario: an annotated tag names its own commit with the ^0 dereference marker
    Given a root commit "n0"
    And an annotated tag "rel" at "n0" with tagger date 100
    When I name "n0" by tags
    Then the rev-name is "rel^0"

  Scenario: a lightweight tag names its own commit by its bare name
    Given a root commit "n0"
    And a lightweight tag "lw" at "n0" with tagger date 100
    When I name "n0" by tags
    Then the rev-name is "lw"

  # --- merge: stepping onto a non-first parent is ^N (generation resets) ---

  Scenario: a commit reached only as a merge's non-first parent is named with ^N
    Given a root commit "base"
    And a commit "sa" with parent "base"
    And a commit "sb1" with parent "base"
    And a commit "sb2" with parent "sb1"
    And a commit "merge" with parents "sb2" and "sa"
    And an annotated tag "mtag" at "merge" with tagger date 100
    When I name "sa" by tags
    Then the rev-name is "mtag^2"

  Scenario: a first-parent step off a ^N name appends ~N before the ^M
    Given a root commit "a0"
    And a root commit "q"
    And a commit "a" with parents "a0" and "q"
    And a root commit "x"
    And a commit "merge" with parents "a" and "x"
    And an annotated tag "mtag" at "merge" with tagger date 100
    When I name "q" by tags
    Then the rev-name is "mtag~1^2"

  Scenario: a long first-parent path beats a short path through a merge parent
    Given a root commit "base"
    And a commit "sa" with parent "base"
    And a commit "sb1" with parent "base"
    And a commit "sb2" with parent "sb1"
    And a commit "merge" with parents "sb2" and "sa"
    And an annotated tag "mtag" at "merge" with tagger date 100
    When I name "base" by tags
    Then the rev-name is "mtag~3"

  # --- tie-break between two tags ---

  Scenario: the nearer of two tags wins, even when the farther one is older
    Given a root commit "x0"
    And a commit "x1" with parent "x0"
    And a commit "x2" with parent "x1"
    And an annotated tag "far" at "x2" with tagger date 1
    And an annotated tag "near" at "x1" with tagger date 999
    When I name "x0" by tags
    Then the rev-name is "near~1"

  Scenario: equidistant tags break the tie in favour of the older tag, not the name
    Given a root commit "base"
    And a commit "left" with parent "base"
    And a commit "right" with parent "base"
    And an annotated tag "aaa" at "right" with tagger date 200
    And an annotated tag "zlate" at "left" with tagger date 100
    When I name "base" by tags
    Then the rev-name is "zlate~1"

  # --- nothing names a commit no tag reaches ---

  Scenario: a commit no tag can reach has no rev-name
    Given a root commit "base"
    And a commit "child" with parent "base"
    And an annotated tag "rel" at "base" with tagger date 5
    When I name "child" by tags
    Then there is no rev-name
