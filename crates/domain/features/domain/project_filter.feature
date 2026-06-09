Feature: Scoping a project listing to a subdirectory
  gitweb's $project_filter (the `pf` parameter) keeps only the projects whose
  path lies under a given subdirectory. The match is by whole path component
  (gitweb's `$path !~ m!^\Qfilter\E/!`): a project directly or deeply under the
  subdirectory is included, but a sibling that merely shares a string prefix is
  not, and a project whose path equals the filter is not either — only a deeper
  component counts.

  Scenario: a project directly under the subdirectory is included
    Given the project filter "group"
    Then "group/alpha.git" is under the filter

  Scenario: a project in a deeper subdirectory is included
    Given the project filter "group"
    Then "group/team/beta.git" is under the filter

  Scenario: a project outside the subdirectory is excluded
    Given the project filter "group"
    Then "other/alpha.git" is not under the filter

  Scenario: a sibling sharing a string prefix is excluded
    Given the project filter "foo"
    Then "foobar.git" is not under the filter

  Scenario: a project whose path equals the filter is excluded
    Given the project filter "group"
    Then "group" is not under the filter

  Scenario: a nested filter matches only its own component path
    Given the project filter "group/team"
    Then "group/team/beta.git" is under the filter
    And "group/teamster.git" is not under the filter
