Feature: Listing a project's branches (heads action)
  The heads use case (gitweb's git_heads / git_get_heads_list) lists the refs
  under refs/heads/, enriches each with its tip commit's age relative to an
  injected request time, and orders them the way gitweb's for-each-ref does:
  most-recently-committed first, with the branch HEAD points at floated above
  any branch sharing its commit time, then by ref name. The branch (or branches)
  whose tip is HEAD's commit is marked current. A branch tip with no commit time
  has an unknown age, and a repository with no branches — including an unborn one
  with no HEAD — lists nothing without failing.

  Background:
    Given the current time is 1000000

  # --- discovery (zero / one / many) ---

  Scenario: a repository with no branches lists no heads
    Given the repository HEAD is the unborn branch "main"
    When I assemble the heads
    Then no heads are listed

  Scenario: a single branch is listed
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 900000
    When I assemble the heads
    Then the listed heads are "main"

  Scenario: branches are ordered by commit recency, newest first
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 100
    And the repository has branch "ancient" committed at 50
    And the repository has branch "newest" committed at 200
    When I assemble the heads
    Then the listed heads are "newest, main, ancient"

  # --- the HEAD-first tiebreak (only when commit times are equal) ---

  Scenario: a newer non-HEAD branch still sorts above the HEAD branch
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 100
    And the repository has branch "feature" committed at 200
    When I assemble the heads
    Then the listed heads are "feature, main"

  Scenario: branches sharing a commit time keep the HEAD branch first
    Given the repository HEAD is branch "main"
    And the repository has branch "zeta" committed at 500
    And the repository has branch "main" committed at 500
    And the repository has branch "alpha" committed at 500
    When I assemble the heads
    Then the listed heads are "main, alpha, zeta"

  # --- current-branch marking (commit-id equality, not just the HEAD ref) ---

  Scenario: the branch HEAD points at is marked current
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 100
    And the repository has branch "side" committed at 200
    When I assemble the heads
    Then the head "main" is current
    And the head "side" is not current

  Scenario: a branch sharing HEAD's commit is also marked current
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 100
    And the repository has branch "release" at the same commit as "main"
    When I assemble the heads
    Then the head "main" is current
    And the head "release" is current

  # --- age, relative to the request time ---

  Scenario: the branch age is measured from the current time
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 999400
    When I assemble the heads
    Then the head "main" shows the age "10 min ago"

  Scenario: a branch whose tip has no commit time has an unknown age
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 0
    When I assemble the heads
    Then the head "main" has no age
