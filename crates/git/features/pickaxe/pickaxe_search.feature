Feature: Pickaxe search through the gix adapter
  gitweb's pickaxe (git_search_changes) runs `git log -S<text> --raw`: it lists
  the commits where the *number of occurrences* of the pattern in some file
  changed against the parent, and — without `--pickaxe-all` — under each commit
  only the paths that *touch* the pattern (the files whose count changed). The
  match is case-sensitive in both modes; with `search_use_regexp` the pattern is a
  case-sensitive POSIX ERE (`--pickaxe-regex`). The walk roots at a base revision
  the caller supplies (gitweb's `$hash`, HEAD by default) and, unlike message
  search, is unpaged — `git log -S` has no count limit. This is the gix adapter
  honouring the Repository port's `pickaxe` operation over deterministic gix-built
  fixtures whose ids, identities, and commit times are pinned.

  The "pickaxe history" fixture is a five-commit chain c1..c5 off a root, HEAD at
  c5, commit times increasing by 100s so newest-first order is fixed. It tracks
  the "banana" occurrence count so commits add it, remove it, and edit a file
  without changing it:

    c1  notes.txt=alpha,         todo.txt="TODO one"          (root)
    c2  + recipe.txt="banana bread"                           banana: 0 -> 1
    c3  recipe.txt="banana cake"                              banana: 1 -> 1 (no change)
    c4  - recipe.txt                                          banana: 1 -> 0 (delete)
    c5  notes.txt="alpha\nbanana", todo.txt="TODO two",
        + extra.txt="banana"                                  banana up in two files, todo untouched-count

  # --- the commits whose occurrence count changed, newest first ---

  Scenario: pickaxe lists every commit that changed the count, newest first
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then 3 commits match
    And matching commit 0 is "c5"
    And matching commit 1 is "c4"
    And matching commit 2 is "c2"

  Scenario: a commit that edits the file without changing the count is not listed
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c3" is not a match

  Scenario: a pattern that never appears matches nothing
    Given a pickaxe history
    When I pickaxe-search for "zucchini"
    Then 0 commits match

  # --- each hit lists exactly its count-changing files ---

  Scenario: a single-file change lists that one file
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c2" changed 1 file
    And commit "c2" changed file "recipe.txt"

  Scenario: the listed blob is the file's post-change blob
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c2" changed file "recipe.txt" holding blob "recipe-v1"

  Scenario: a commit that changed the count in several files lists each of them
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c5" changed 2 files
    And commit "c5" changed file "notes.txt"
    And commit "c5" changed file "extra.txt"

  Scenario: a touched file whose count did not change is not listed (no --pickaxe-all)
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c5" did not change file "todo.txt"

  # --- a deletion that changed the count is reported as a deletion ---

  Scenario: removing the file is a change, reported as a deletion
    Given a pickaxe history
    When I pickaxe-search for "banana"
    Then commit "c4" changed 1 file
    And commit "c4" deleted file "recipe.txt"

  # --- case sensitivity: pickaxe never folds case ---

  Scenario: pickaxe is case-sensitive
    Given a pickaxe history
    When I pickaxe-search for "BANANA"
    Then 0 commits match

  # --- the regexp toggle (--pickaxe-regex), still case-sensitive ---

  Scenario: a regexp pickaxe matches by pattern
    Given a pickaxe history
    When I regexp-pickaxe-search for "ban.na"
    Then 3 commits match
    And matching commit 0 is "c5"

  Scenario: the same pattern as a fixed string treats the dot literally and matches nothing
    Given a pickaxe history
    When I pickaxe-search for "ban.na"
    Then 0 commits match

  # --- non-HEAD rooting: the walk starts from the requested revision ---

  Scenario: a pickaxe rooted at an older commit ignores commits it cannot reach
    Given a pickaxe history
    When I pickaxe-search for "banana" rooted at "c2"
    Then 1 commit matches
    And matching commit 0 is "c2"
