Feature: Listing projects from a projects-list file through the gix adapter
  Instead of scanning the project root, gitweb can read a `$projects_list` file
  that names each project (and optionally its owner), URL-encoded. This is the
  gix adapter in file mode: it lists exactly the named projects that are real
  repositories under the root (gitweb's `check_export_ok`, which still requires
  `check_head_link`), and the owner named in the file wins over git config.

  # --- list(): zero / one / many ---

  Scenario: a file naming no existing repository lists nothing
    Given a project root
    And a projects-list file:
      """
      ghost.git
      """
    When I list the projects
    Then 0 projects are listed

  Scenario: a file naming one repository lists it
    Given a project root
    And a repository "solo.git"
    And a projects-list file:
      """
      solo.git
      """
    When I list the projects
    Then 1 project is listed
    And the project "solo.git" is listed

  Scenario: a file naming several repositories lists them in file order
    Given a project root
    And a repository "alpha.git"
    And a repository "beta.git"
    And a projects-list file:
      """
      beta.git
      alpha.git
      """
    When I list the projects
    Then 2 projects are listed
    And the project "alpha.git" is listed
    And the project "beta.git" is listed

  # --- file-mode edges ---

  Scenario: a repository on disk but absent from the file is not listed
    Given a project root
    And a repository "listed.git"
    And a repository "unlisted.git"
    And a projects-list file:
      """
      listed.git
      """
    When I list the projects
    Then 1 project is listed
    And the project "listed.git" is listed

  Scenario: a named repository that does not exist is skipped
    Given a project root
    And a repository "real.git"
    And a projects-list file:
      """
      real.git
      ghost.git
      """
    When I list the projects
    Then 1 project is listed
    And the project "real.git" is listed

  Scenario: a URL-encoded path is decoded to its repository
    Given a project root
    And a repository "group/nested.git"
    And a projects-list file:
      """
      group%2Fnested.git
      """
    When I list the projects
    Then 1 project is listed
    And the project "group/nested.git" is listed

  Scenario: a blank line in the file is ignored
    Given a project root
    And a repository "solo.git"
    And a projects-list file:
      """

      solo.git

      """
    When I list the projects
    Then 1 project is listed
    And the project "solo.git" is listed

  # --- owner precedence: the file wins over git config ---

  Scenario: the owner is taken from the projects-list file
    Given a project root
    And a repository "solo.git"
    And a projects-list file:
      """
      solo.git Ada+Lovelace
      """
    When I read the metadata of "solo.git"
    Then the owner is "Ada Lovelace"

  Scenario: the file owner overrides the gitweb config owner
    Given a project root
    And a repository "solo.git"
    And "solo.git" has gitweb config "owner" set to "Config Owner"
    And a projects-list file:
      """
      solo.git File+Owner
      """
    When I read the metadata of "solo.git"
    Then the owner is "File Owner"
