Feature: The patches use case (a commit range as a numbered format-patch stream)
  gitweb's `git_patches` is `git_commitdiff(-format => 'patch')` — the range form,
  streamed from `git format-patch -M --encoding=utf8 --stdout -<patch_max> -n --root
  <hash>`: the most recent `patch_max` commits reachable from the tip, oldest-first,
  each `Subject` numbered `[PATCH i/N]` where N is the count actually emitted. It
  reuses the patch mail the single form builds — this use case adds the range walk,
  the `patch_max` cap, and the numbering. It gates on the `patches` feature the way
  gitweb does (`die_error(403, "Patch view not allowed") unless $patch_max`) and
  refuses a non-commit with the 404 "Unknown commit object".

  Scenario: A range emits one numbered mail per commit, oldest-first
    Given a patch series authored by "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And a patch commit "root" with subject "Add the engine" creating "engine.txt"
    And a patch commit "tune" with subject "Tune the engine" creating "tune.txt"
    When I assemble the patches for "HEAD" with limit 16 and version "2.54.0"
    Then the patches stream has a line "Subject: [PATCH 1/2] Add the engine"
    And the patches stream has a line "Subject: [PATCH 2/2] Tune the engine"
    And "Add the engine" comes before "Tune the engine" in the patches stream

  Scenario: A single-commit range numbers the one mail [PATCH 1/1]
    Given a patch series authored by "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And a patch commit "root" with subject "Add the engine" creating "engine.txt"
    When I assemble the patches for "HEAD" with limit 16 and version "2.54.0"
    Then the patches stream has a line "Subject: [PATCH 1/1] Add the engine"

  Scenario: The patch_max cap keeps the most recent commits and counts them
    Given a patch series authored by "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And a patch commit "first" with subject "First" creating "a.txt"
    And a patch commit "second" with subject "Second" creating "b.txt"
    And a patch commit "third" with subject "Third" creating "c.txt"
    When I assemble the patches for "HEAD" with limit 2 and version "2.54.0"
    Then the patches stream has a line "Subject: [PATCH 1/2] Second"
    And the patches stream has a line "Subject: [PATCH 2/2] Third"
    And the patches stream does not contain "First"

  Scenario: The patches feature being off forbids the view
    Given a patch series authored by "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And a patch commit "root" with subject "Add the engine" creating "engine.txt"
    When I assemble the patches for "HEAD" with limit 0 and version "2.54.0"
    Then assembling the patches fails with "Patch view not allowed"

  Scenario: A revision that is not a commit is unknown
    Given a patch series authored by "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And a patch commit "root" with subject "Add the engine" creating "engine.txt"
    And the patch series tip is not a commit
    When I assemble the patches for "HEAD" with limit 16 and version "2.54.0"
    Then assembling the patches fails with "Unknown commit object"
