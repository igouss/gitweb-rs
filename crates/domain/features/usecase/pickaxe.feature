Feature: Assembling pickaxe-search results
  gitweb's git_search dispatches a pickaxe search to git_search_changes: it runs
  `git log -S<text> --raw` from the base revision and lists, newest-first, every
  commit whose occurrence count of the pattern changed in some file, and under
  each — without `--pickaxe-all` — only the files whose count changed, each
  linking to its post-change blob. A deleted file is skipped (gitweb's
  `next if is_deleted`), having no blob to link, though its commit still shows.
  Unlike message search, pickaxe is unpaged. This use case orchestrates that over
  the Repository port: it gates on `search` then `pickaxe` (403 each when off),
  validates the pattern (400 on a bad regexp), resolves the base (HEAD by default,
  404 "Unknown commit object" when it names no commit), and maps each match to a
  row. The counting, rooting, and per-commit file set are the adapter's
  conformance; the fake returns the configured matches so the assembly's own
  behaviour — gating, base resolution, the deletion skip, row assembly — is what is
  driven here.

  Background:
    Given a commit "base" at epoch 1000 by "Tester" titled "pickaxe base"
    And the repository HEAD is at commit "base"

  # --- the two feature gates, in gitweb's order ---

  Scenario: a pickaxe is forbidden when the search feature is disabled
    When I assemble a search-disabled pickaxe for "needle"
    Then the pickaxe is forbidden as "Search is disabled"

  Scenario: a pickaxe is forbidden when the pickaxe feature is disabled
    When I assemble a pickaxe-disabled pickaxe for "needle"
    Then the pickaxe is forbidden as "Pickaxe search is disabled"

  # --- pattern validation and base resolution ---

  Scenario: a malformed regular expression is rejected
    When I assemble a regexp pickaxe for "a(b"
    Then the pickaxe is invalid

  Scenario: a base revision that names no commit is an unknown commit object
    When I assemble a pickaxe for "needle" rooted at "nope"
    Then the pickaxe reports an unknown commit object

  # --- the page header is the base commit's subject ---

  Scenario: the header shows the base commit subject
    When I assemble a pickaxe for "needle"
    Then the pickaxe header title is "pickaxe base"
    And the pickaxe roots at "base"

  # --- zero / one / many matching commits ---

  Scenario: a pickaxe with no matches lists no commits
    When I assemble a pickaxe for "needle"
    Then the pickaxe lists 0 commits

  Scenario: one matching commit is listed with its changed file
    Given a pickaxe hit "c1" at epoch 1100 by "Ada Lovelace" titled "Add the needle"
    And the pickaxe hit "c1" changed file "src/a.txt" at blob "blobA"
    When I assemble a pickaxe for "needle"
    Then the pickaxe lists 1 commit
    And pickaxe commit 0 is "c1"
    And pickaxe commit "c1" lists 1 file
    And pickaxe commit "c1" file 0 is "src/a.txt" linking blob "blobA"

  Scenario: several matching commits keep the order the port returned (newest first)
    Given a pickaxe hit "c2" at epoch 1200 by "Ada" titled "Remove the needle"
    And the pickaxe hit "c2" changed file "b.txt" at blob "blobB"
    And a pickaxe hit "c1" at epoch 1100 by "Ada" titled "Add the needle"
    And the pickaxe hit "c1" changed file "a.txt" at blob "blobA"
    When I assemble a pickaxe for "needle"
    Then the pickaxe lists 2 commits
    And pickaxe commit 0 is "c2"
    And pickaxe commit 1 is "c1"

  Scenario: a commit that changed the count in several files links each of them in order
    Given a pickaxe hit "c1" at epoch 1100 by "Ada" titled "Spread the needle"
    And the pickaxe hit "c1" changed file "a.txt" at blob "blobA"
    And the pickaxe hit "c1" changed file "b.txt" at blob "blobB"
    When I assemble a pickaxe for "needle"
    Then pickaxe commit "c1" lists 2 files
    And pickaxe commit "c1" file 0 is "a.txt" linking blob "blobA"
    And pickaxe commit "c1" file 1 is "b.txt" linking blob "blobB"

  # --- the deletion skip: a deleted file has no blob to link ---

  Scenario: a deleted file is skipped but its still-present sibling is listed
    Given a pickaxe hit "c1" at epoch 1100 by "Ada" titled "Swap the needle"
    And the pickaxe hit "c1" changed file "kept.txt" at blob "blobK"
    And the pickaxe hit "c1" deleted file "gone.txt"
    When I assemble a pickaxe for "needle"
    Then pickaxe commit "c1" lists 1 file
    And pickaxe commit "c1" file 0 is "kept.txt" linking blob "blobK"

  Scenario: a commit whose only count change deleted the file still shows, with no file links
    Given a pickaxe hit "c1" at epoch 1100 by "Ada" titled "Delete the needle"
    And the pickaxe hit "c1" deleted file "gone.txt"
    When I assemble a pickaxe for "needle"
    Then the pickaxe lists 1 commit
    And pickaxe commit "c1" lists 0 files

  # --- the row's author is chopped to gitweb's 15 characters ---

  Scenario: the row author is chopped to fifteen characters
    Given a pickaxe hit "c1" at epoch 1100 by "Maximilian Alexander Throckmorton" titled "Edit"
    And the pickaxe hit "c1" changed file "a.txt" at blob "blobA"
    When I assemble a pickaxe for "needle"
    Then pickaxe commit "c1" author shortens to "Maximilian Alexander... "
    And pickaxe commit "c1" full author is "Maximilian Alexander Throckmorton"
