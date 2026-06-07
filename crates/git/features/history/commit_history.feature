Feature: Walking commit history through the gix adapter
  gitweb's parse_commits is `git rev-list --header --max-count=N --skip=S <start> --
  [path]`: the commits reachable from a start revision, newest first by commit
  time, windowed by skip/limit, optionally limited to commits that touch a path.
  This is the gix adapter honouring the Repository port's `history` operation over
  deterministic, gix-built fixtures whose object ids and commit times are pinned,
  so ordering and counts are exact.

  The "line" fixture is a five-commit chain off a root, with two files: a.txt is
  rewritten by some commits and left untouched by others, b.txt is added partway
  through. Commit times strictly increase along the chain, so newest-first order
  is c5, c4, c3, c2, c1, and the path-limited walk against a.txt is exactly the
  subset that changed it: c4, c2, c1.

  # --- many / one: the reachable set, newest first ---

  Scenario: history from the tip lists every reachable commit, newest first
    Given a line of history
    When I read the history from "c5"
    Then 5 commits are listed
    And commit 0 is "c5"
    And commit 1 is "c4"
    And commit 2 is "c3"
    And commit 3 is "c2"
    And commit 4 is "c1"

  Scenario: history from the root lists exactly that one commit
    Given a line of history
    When I read the history from "c1"
    Then 1 commit is listed
    And commit 0 is "c1"

  # --- a root commit (no parents) sits at the end of history ---

  Scenario: the last commit reachable from the tip is the parentless root
    Given a line of history
    When I read the history from "c5"
    Then commit 4 is "c1"
    And commit 4 has 0 parents

  # --- pagination: first page, last partial page, and past the end (zero) ---

  Scenario Outline: paging the history walks the window the page selects
    Given a line of history
    When I read page <page> of size 2 from "c5"
    Then <count> commits are listed

    Examples:
      | page | count |
      | 0    | 2     |
      | 1    | 2     |
      | 2    | 1     |
      | 3    | 0     |

  Scenario: the first page holds the newest commits in order
    Given a line of history
    When I read page 0 of size 2 from "c5"
    Then 2 commits are listed
    And commit 0 is "c5"
    And commit 1 is "c4"

  Scenario: the last partial page holds the lone remaining root
    Given a line of history
    When I read page 2 of size 2 from "c5"
    Then 1 commit is listed
    And commit 0 is "c1"

  # --- path filter: selects the subset that changed the path ---

  Scenario: limiting to a path keeps only commits that changed it
    Given a line of history
    When I read the history from "c5" touching "a.txt"
    Then 3 commits are listed
    And commit 0 is "c4"
    And commit 1 is "c2"
    And commit 2 is "c1"

  Scenario: limiting to a path a commit added keeps just that commit
    Given a line of history
    When I read the history from "c5" touching "b.txt"
    Then 2 commits are listed
    And commit 0 is "c5"
    And commit 1 is "c3"

  # --- path filter that matches nothing: the empty (zero) result ---

  Scenario: limiting to a path that never existed lists nothing
    Given a line of history
    When I read the history from "c5" touching "nope.txt"
    Then 0 commits are listed

  # --- multi-parent (octopus) ancestry ordering ---

  Scenario: an octopus merge is listed first, ahead of all its parents
    Given an octopus merge
    When I read the history from "m"
    Then 5 commits are listed
    And commit 0 is "m"
    And commit 0 has 3 parents
    And commit 0 is a merge
    And commit 1 is "p3"
    And commit 2 is "p2"
    And commit 3 is "p1"
    And commit 4 is "r"
