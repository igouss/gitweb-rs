Feature: Assembling the projects-list page
  The project_list use case (gitweb's git_project_list) lists the projects under
  the root, fills each with its display metadata, applies the sort order — the
  request's, or the configured default — and dies "404 No projects found" when
  the root is empty. It computes each project's last-change age relative to an
  injected request time, so a project with no commits has no age.

  # --- discovery (zero / one / many) ---

  Scenario: an empty project root has no list
    When I assemble the project list
    Then assembling fails as not found

  Scenario: a single project is listed
    Given the store has project "only.git"
    When I assemble the project list
    Then the listed projects are "only.git"

  Scenario: the list defaults to ordering by project path
    Given the store has project "zlib.git"
    And the store has project "make.git"
    And the store has project "acl.git"
    When I assemble the project list
    Then the listed projects are "acl.git, make.git, zlib.git"

  # --- the order parameter ---

  Scenario: an explicit order overrides the default
    Given the store has project "acl.git" last changed at 1000
    And the store has project "zlib.git" last changed at 3000
    When I assemble the project list ordered by "age"
    Then the listed projects are "zlib.git, acl.git"

  Scenario: an unknown order is rejected
    Given the store has project "a.git"
    When I assemble the project list ordered by "sideways"
    Then assembling fails as invalid

  # --- per-row metadata ---

  Scenario: a project's description and owner appear on its row
    Given the store has project "tool.git" described as "A small tool" owned by "Ann"
    When I assemble the project list
    Then the project "tool.git" shows description "A small tool"
    And the project "tool.git" shows owner "Ann"

  # --- last-change age, relative to the request time ---

  Scenario: the last-change age is measured from the current time
    Given the current time is 1000
    And the store has project "live.git" last changed at 400
    When I assemble the project list
    Then the project "live.git" shows the age "10 min ago"

  Scenario: a project with no commits has no age
    Given the store has project "void.git" with no commits
    When I assemble the project list
    Then the project "void.git" has no age

  # --- the project filter (pf subdirectory scoping) ---

  Scenario: a filter scopes the list to its subdirectory
    Given the store has project "group/alpha.git"
    And the store has project "group/beta.git"
    And the store has project "other/gamma.git"
    When I assemble the project list filtered by "group"
    Then the listed projects are "group/alpha.git, group/beta.git"

  Scenario: a filter matching no project is still the not-found error
    Given the store has project "group/alpha.git"
    When I assemble the project list filtered by "elsewhere"
    Then assembling fails as not found

  # --- the forks feature (fold forks under their parent) ---

  Scenario: without the forks feature every project is listed flat
    Given the store has project "repo.git"
    And the store has project "repo/fork.git"
    When I assemble the project list
    Then the listed projects are "repo.git, repo/fork.git"
    And the listing has no fork column

  Scenario: the forks feature folds a fork under its parent
    Given the forks feature is enabled
    And the store has project "repo.git"
    And the store has project "repo/fork.git"
    When I assemble the project list
    Then the listed projects are "repo.git"
    And the listing has the fork column
    And the project "repo.git" reports 1 fork

  Scenario: the forks feature reports a project's full fork count
    Given the forks feature is enabled
    And the store has project "repo.git"
    And the store has project "repo/one.git"
    And the store has project "repo/two.git"
    When I assemble the project list
    Then the listed projects are "repo.git"
    And the project "repo.git" reports 2 forks

  Scenario: a fork-less project reports no forks even with the feature on
    Given the forks feature is enabled
    And the store has project "solo.git"
    When I assemble the project list
    Then the project "solo.git" reports 0 forks
