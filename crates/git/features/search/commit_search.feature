Feature: Searching commit history through the gix adapter
  gitweb's search splits into facets that each list matching commits. Message,
  author, and committer search are `git log --grep= / --author= / --committer=`
  with `--regexp-ignore-case --fixed-strings`: a case-insensitive substring over
  the commit message or the matched identity. Pickaxe search is `git log -M -S`:
  the commits where the number of occurrences of the pattern changed in some
  file — a case-sensitive, count-aware match, not mere presence. The pattern is
  fixed-string by default but a (case-insensitive) POSIX extended regular
  expression when gitweb's `search_use_regexp` is set, and the walk roots at a
  base revision the caller supplies — gitweb's `$hash`, the current view, HEAD by
  default. This is the gix adapter honouring the Repository port's `search`
  operation over deterministic gix-built fixtures whose ids, identities, and
  commit times are pinned.

  (The `grep` facet — `git grep` listing file/line hits at one revision — returns
  lines, not commits, so it is a separate capability and not part of this port.)

  The "searchable history" fixture is a five-commit chain c1..c5 off a root,
  HEAD at c5, commit times increasing by 100s so newest-first order is fixed:

    c1  author Ada,   committer Ada    "Initial import"     notes.txt=alpha
    c2  author Grace, committer Ada    "Add banana recipe"  + recipe.txt=banana bread
    c3  author Ada,   committer Ada    "Fix recipe typo"    recipe.txt=banana cake
    c4  author Linus, committer Ada    "Remove recipe"      - recipe.txt
    c5  author Ada,   committer Grace  "Update notes"       notes.txt=alpha+beta

  # --- message search: case-insensitive substring of the commit message ---

  Scenario: a message substring lists every commit whose message contains it, newest first
    Given a searchable history
    When I search messages for "recipe"
    Then 3 commits are found
    And found commit 0 is "c4"
    And found commit 1 is "c3"
    And found commit 2 is "c2"

  Scenario: a message search is case-insensitive
    Given a searchable history
    When I search messages for "fix"
    Then 1 commit is found
    And found commit 0 is "c3"

  Scenario: a message substring that occurs in no commit finds nothing
    Given a searchable history
    When I search messages for "xylophone"
    Then 0 commits are found

  # --- regexp toggle: the pattern as a POSIX extended regular expression ---

  Scenario: a regexp message search matches by pattern, not literally
    Given a searchable history
    When I regexp-search messages for "rec.pe"
    Then 3 commits are found
    And found commit 0 is "c4"
    And found commit 1 is "c3"
    And found commit 2 is "c2"

  Scenario: the same pattern as a fixed string treats the dot literally and matches nothing
    Given a searchable history
    When I search messages for "rec.pe"
    Then 0 commits are found

  Scenario: a regexp message search is case-insensitive too
    Given a searchable history
    When I regexp-search messages for "F[il]X"
    Then 1 commit is found
    And found commit 0 is "c3"

  # --- non-HEAD rooting: the walk starts from the requested revision ---

  Scenario: a search rooted at an older commit ignores commits it cannot reach
    Given a searchable history
    When I search messages for "recipe" rooted at "c3"
    Then 2 commits are found
    And found commit 0 is "c3"
    And found commit 1 is "c2"

  # --- author search: the matched "Name <email>" identity ---

  Scenario: author search matches many commits by the same author
    Given a searchable history
    When I search authors for "ada"
    Then 3 commits are found
    And found commit 0 is "c5"
    And found commit 1 is "c3"
    And found commit 2 is "c1"

  Scenario: author search matches the name part of the identity
    Given a searchable history
    When I search authors for "Hopper"
    Then 1 commit is found
    And found commit 0 is "c2"

  Scenario: author search finds nothing for an unknown identity
    Given a searchable history
    When I search authors for "nobody"
    Then 0 commits are found

  # --- committer search is distinct from author search ---

  Scenario: committer search matches the committing identity, not the author
    Given a searchable history
    When I search committers for "grace"
    Then 1 commit is found
    And found commit 0 is "c5"

  Scenario: an author who never committed is found by author but not committer search
    Given a searchable history
    When I search authors for "linus"
    Then 1 commit is found
    And found commit 0 is "c4"

  Scenario: that same identity yields nothing as a committer
    Given a searchable history
    When I search committers for "linus"
    Then 0 commits are found

  # --- pickaxe: commits where the pattern's occurrence count changed ---

  Scenario: pickaxe lists the commits that added or removed the pattern
    Given a searchable history
    When I pickaxe-search for "banana"
    Then 2 commits are found
    And found commit 0 is "c4"
    And found commit 1 is "c2"

  Scenario: pickaxe ignores a commit that touches the file without changing the count
    Given a searchable history
    When I pickaxe-search for "alpha"
    Then 1 commit is found
    And found commit 0 is "c1"

  Scenario: pickaxe is case-sensitive, unlike message search
    Given a searchable history
    When I pickaxe-search for "BANANA"
    Then 0 commits are found

  Scenario: pickaxe finds nothing for a pattern that never appears
    Given a searchable history
    When I pickaxe-search for "zucchini"
    Then 0 commits are found

  # --- pagination boundaries over a multi-match result ---

  Scenario Outline: paging a message search walks the window the page selects
    Given a searchable history
    When I search messages for "recipe" on page <page> of size 2
    Then <count> commits are found

    Examples:
      | page | count |
      | 0    | 2     |
      | 1    | 1     |
      | 2    | 0     |

  Scenario: the first page of a message search holds the newest matches in order
    Given a searchable history
    When I search messages for "recipe" on page 0 of size 2
    Then 2 commits are found
    And found commit 0 is "c4"
    And found commit 1 is "c3"
