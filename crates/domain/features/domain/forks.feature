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
