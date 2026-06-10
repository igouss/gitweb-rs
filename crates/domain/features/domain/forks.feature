Feature: Detecting forks among projects
  gitweb groups a project and its forks: forks of `repo.git` live in the
  `repo/` directory, as `repo/whatever.git`. filter_forks_from_projects_list
  builds a prefix tree of project paths (with the trailing `.git` stripped) and
  removes any project that sits under a shorter project's directory, attaching
  it to that parent as a fork.

  Matching is by whole path component, not by string prefix: `foobar.git` is not
  a fork of `foo.git`, and a project under a bare directory is only a fork when a
  project actually owns that directory.

  Scenario: a flat list has no forks
    Given a project path "alpha.git"
    And a project path "beta.git"
    When I partition forks
    Then 2 top-level projects remain
    And "alpha.git" is a top-level project
    And "beta.git" is a top-level project
    And "alpha.git" has 0 forks

  Scenario: a project with one fork keeps the fork off the top level
    Given a project path "repo.git"
    And a project path "repo/fork.git"
    When I partition forks
    Then 1 top-level project remains
    And "repo.git" is a top-level project
    And "repo/fork.git" is not a top-level project
    And "repo.git" has 1 fork
    And "repo.git" has the fork "repo/fork.git"

  Scenario: a project with several forks collects them all
    Given a project path "repo.git"
    And a project path "repo/one.git"
    And a project path "repo/two.git"
    And a project path "repo/three.git"
    When I partition forks
    Then 1 top-level project remains
    And "repo.git" has 3 forks
    And "repo.git" has the fork "repo/one.git"
    And "repo.git" has the fork "repo/two.git"
    And "repo.git" has the fork "repo/three.git"

  Scenario: a longer name sharing a string prefix is not a fork
    Given a project path "foo.git"
    And a project path "foobar.git"
    When I partition forks
    Then 2 top-level projects remain
    And "foo.git" has 0 forks

  Scenario: projects sharing a directory but with no owning parent are independent
    Given a project path "group/one.git"
    And a project path "group/two.git"
    When I partition forks
    Then 2 top-level projects remain
    And "group/one.git" has 0 forks
    And "group/two.git" has 0 forks

  # --- the forks-of subdirectory (gitweb's git_forks filter) ---
  # The forks of `repo.git` live in `repo/`, so the forks action scopes the
  # listing to the project path with one trailing `.git` removed.

  Scenario: a bare project's forks live in its name without the .git suffix
    Given the project "repo.git"
    Then its forks live in the subdirectory "repo"

  Scenario: a project path without a .git suffix is its own forks subdirectory
    Given the project "repo"
    Then its forks live in the subdirectory "repo"

  Scenario: only one trailing .git is stripped to find the forks subdirectory
    Given the project "nested/work.git"
    Then its forks live in the subdirectory "nested/work"

  # --- the fork container (gitweb's name-side guards before the -d test) ---
  # A project is fork-capable only when, with one trailing `.git` stripped, the
  # remainder is a real bare-repo directory name: not the `.git` repo itself and
  # not a non-bare `repo/.git` working tree. That remainder is the directory the
  # store must then confirm exists on disk.

  Scenario: a bare project's fork container is its name without the .git suffix
    Given the project "repo.git"
    Then its fork container is "repo"

  Scenario: a project path without a .git suffix is its own fork container
    Given the project "repo"
    Then its fork container is "repo"

  Scenario: a nested bare project's fork container keeps its parent directories
    Given the project "nested/work.git"
    Then its fork container is "nested/work"

  Scenario: a non-bare repository can never parent forks
    Given the project "repo/.git"
    Then it has no fork container

  Scenario: the bare .git repository itself can never parent forks
    Given the project ".git"
    Then it has no fork container
