Feature: Content grep at one revision through the gix adapter
  gitweb's `git_search_files` greps the files of one revision with `git grep -n
  -z <pattern>` in one of two modes — `-F` (literal, case-sensitive) or `-E -i`
  (case-insensitive regexp) — listing each text-file line that matches (its path
  and 1-based number) and each binary file whose bytes match (with no line text).
  gix has no `git grep`,
  so the adapter walks the revision's tree, reads each *regular file* blob, and
  delegates the per-file matching to the domain rule — symlinks and gitlinks are
  not blobs git greps, so they are skipped. Matches come out grouped by file in
  tree order; the listing is capped at gitweb's limit, reporting `trimmed` when
  the cap drops further matches. This is the gix adapter honouring the Repository
  port's `grep` operation over deterministic gix-built fixtures.

  The "tree of mixed files" fixture is one commit whose tree holds, in path
  order:

    alpha.txt    text   "alpha" / "beta" / "alpha gamma"
    beta.txt     text   "beta"
    data.bin     binary bytes (a NUL) that contain "alpha"
    latin.txt    text   latin1 "café noir" (not valid UTF-8, no NUL)
    link         symlink whose target text is "alpha.txt"
    modz         gitlink (submodule) pointer
    run.sh       executable text "alpha script"
    sub/deep.txt text   "alpha in subdir"

  # --- zero / one / many over a tree ---

  Scenario: a pattern in no file finds nothing
    Given a tree of mixed files
    When I grep "zucchini" at the commit
    Then 0 grep matches are found

  Scenario: a pattern in one file on one line is the only match
    Given a tree of mixed files
    When I grep "gamma" at the commit
    Then 1 grep match is found
    And grep match 0 is line 3 "alpha gamma" in "alpha.txt"

  Scenario: a pattern is listed per file and per line, grouped by file in tree order
    Given a tree of mixed files
    When I grep "alpha" at the commit
    Then 5 grep matches are found
    And grep match 0 is line 1 "alpha" in "alpha.txt"
    And grep match 1 is line 3 "alpha gamma" in "alpha.txt"
    And grep match 2 is binary file "data.bin"
    And grep match 3 is line 1 "alpha script" in "run.sh"
    And grep match 4 is line 1 "alpha in subdir" in "sub/deep.txt"

  Scenario: a pattern in several files is grouped one match per file
    Given a tree of mixed files
    When I grep "beta" at the commit
    Then 2 grep matches are found
    And grep match 0 is line 2 "beta" in "alpha.txt"
    And grep match 1 is line 1 "beta" in "beta.txt"

  # --- binary files and non-UTF-8 content over real blobs ---

  Scenario: a binary file whose bytes contain the pattern is reported with no text
    Given a tree of mixed files
    When I grep "binary" at the commit
    Then 1 grep match is found
    And grep match 0 is binary file "data.bin"

  Scenario: a non-UTF-8 line matches and is decoded through the fallback encoding
    Given a tree of mixed files
    When I grep "noir" at the commit
    Then 1 grep match is found
    And grep match 0 is line 1 "café noir" in "latin.txt"

  # --- regexp mode (-E -i): case-insensitive matching over real blobs ---

  Scenario: a regexp grep matches case-insensitively across the tree
    Given a tree of mixed files
    When I grep regexp "GAMMA" at the commit
    Then 1 grep match is found
    And grep match 0 is line 3 "alpha gamma" in "alpha.txt"

  Scenario: a regexp metacharacter matches as a regular expression over real blobs
    Given a tree of mixed files
    When I grep regexp "alpha g.mma" at the commit
    Then 1 grep match is found
    And grep match 0 is line 3 "alpha gamma" in "alpha.txt"

  # --- symlinks and gitlinks are not grepped ---

  Scenario: a symlink is skipped even though its target text contains the pattern
    Given a tree of mixed files
    When I grep "alpha.txt" at the commit
    Then 0 grep matches are found

  # --- the revision is a tree-ish: a tree id works like a commit id ---

  Scenario: grepping the tree id yields the same matches as the commit
    Given a tree of mixed files
    When I grep "gamma" at the tree
    Then 1 grep match is found
    And grep match 0 is line 3 "alpha gamma" in "alpha.txt"

  # --- the 1000-match trim boundary ---

  Scenario: a result filling the cap exactly is not trimmed
    Given a file with as many matching lines as the cap allows
    When I grep "x" at the commit
    Then the result fills the cap exactly
    And the listing is not trimmed

  Scenario: one match beyond the cap trims the listing to the cap
    Given a file with one matching line beyond the cap
    When I grep "x" at the commit
    Then the result fills the cap exactly
    And the listing is trimmed
