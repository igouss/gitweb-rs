Feature: Discovering and opening projects through the gix ProjectStore adapter
  The ProjectStore port is a frozen contract; this is the gix adapter honouring
  it over a real, deterministic project root laid out on disk (bare repositories
  built with gix, no git binary). A directory is a project exactly when it looks
  like a git repository — gitweb's check_head_link: a HEAD is present. Export
  gates (export-ok marker, strict export, auth hook) and per-project metadata are
  layered on by a later slice, not here.

  Discovery walks the root for these repositories; opening resolves one by its
  store-relative name into a working Repository, validating the name first so a
  path-traversal attempt can never escape the root.

  # --- list(): zero / one / many ---

  Scenario: an empty project root has no projects
    Given an empty project root
    When I list the projects
    Then 0 projects are listed

  Scenario: a root with a single repository lists it
    Given a project root containing repository "solo.git"
    When I list the projects
    Then 1 project is listed
    And the project "solo.git" is listed

  Scenario: a root with several repositories lists them all
    Given a project root containing repository "alpha.git"
    And the root also contains repository "beta.git"
    And the root also contains repository "gamma.git"
    When I list the projects
    Then 3 projects are listed
    And the project "alpha.git" is listed
    And the project "beta.git" is listed
    And the project "gamma.git" is listed

  # --- list(): discovery edges ---

  Scenario: a nested repository is found by its store-relative path
    Given a project root containing repository "group/nested.git"
    When I list the projects
    Then 1 project is listed
    And the project "group/nested.git" is listed

  Scenario: a directory that is not a repository is ignored
    Given a project root containing repository "real.git"
    And the root also contains a plain directory "notes"
    When I list the projects
    Then 1 project is listed
    And the project "real.git" is listed

  Scenario: discovery does not descend into a repository
    Given a project root containing repository "outer.git"
    When I list the projects
    Then 1 project is listed
    And the project "outer.git" is listed

  # --- open(): success ---

  Scenario: opening a known project yields a working repository
    Given a project root containing repository "solo.git"
    When I open the project "solo.git"
    Then the project opens successfully
    And the opened repository has 1 reference under "refs/heads/"

  Scenario: opening a nested project yields a working repository
    Given a project root containing repository "group/nested.git"
    When I open the project "group/nested.git"
    Then the project opens successfully

  # --- open(): the missing and unsafe edges ---

  Scenario: opening a name that does not exist fails as not found
    Given an empty project root
    When I open the project "ghost.git"
    Then opening fails as not found

  Scenario: a path-traversal name is rejected as invalid
    Given an empty project root
    When I open the project "../etc"
    Then opening fails as invalid

  Scenario: an absolute name is rejected as invalid
    Given an empty project root
    When I open the project "/etc/passwd"
    Then opening fails as invalid
