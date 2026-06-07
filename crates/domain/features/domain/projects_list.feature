Feature: Parsing the projects-list file lines
  When gitweb is given a projects-list file, each line names one project and,
  optionally, its owner — both URL-encoded and split on whitespace, as in
  `git%2Fgit.git Linus+Torvalds`. A blank line carries no project.

  Scenario: a blank line carries no project
    Given the projects-list line ""
    When I parse the projects-list line
    Then the line carries no project

  Scenario: a whitespace-only line carries no project
    Given the projects-list line "   "
    When I parse the projects-list line
    Then the line carries no project

  Scenario: a line with only a path has no owner
    Given the projects-list line "solo.git"
    When I parse the projects-list line
    Then the parsed path is "solo.git"
    And the parsed line has no owner

  Scenario: a line with a path and owner carries both
    Given the projects-list line "solo.git Ada"
    When I parse the projects-list line
    Then the parsed path is "solo.git"
    And the parsed owner is "Ada"

  Scenario: the path and owner are URL-decoded
    Given the projects-list line "git%2Fgit.git Linus+Torvalds"
    When I parse the projects-list line
    Then the parsed path is "git/git.git"
    And the parsed owner is "Linus Torvalds"

  Scenario: leading and repeated whitespace is collapsed
    Given the projects-list line "   spaced.git    The+Owner   "
    When I parse the projects-list line
    Then the parsed path is "spaced.git"
    And the parsed owner is "The Owner"
