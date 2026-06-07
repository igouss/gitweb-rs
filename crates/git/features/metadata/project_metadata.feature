Feature: Per-project metadata through the gix ProjectStore adapter
  gitweb shows each repository's description, owner, category and clone URLs,
  reading each either from a file in the repository (`description`, `category`,
  `cloneurl`) or, when that file is absent, from a `gitweb.*` git config value
  (`git_get_file_or_project_config`). This is the gix adapter resolving that
  metadata over real repositories laid out on disk.

  The file always wins over config; a value present in neither is simply absent.

  # --- description: file, config, precedence, absent ---

  Scenario: a description is read from the description file
    Given a project root containing repository "solo.git"
    And "solo.git" has the description file "A neat little project"
    When I read the metadata of "solo.git"
    Then the description is "A neat little project"

  Scenario: a description falls back to gitweb config when there is no file
    Given a project root containing repository "solo.git"
    And "solo.git" has no description file
    And "solo.git" has gitweb config "description" set to "Configured description"
    When I read the metadata of "solo.git"
    Then the description is "Configured description"

  Scenario: the description file wins over the gitweb config value
    Given a project root containing repository "solo.git"
    And "solo.git" has the description file "From the file"
    And "solo.git" has gitweb config "description" set to "From the config"
    When I read the metadata of "solo.git"
    Then the description is "From the file"

  Scenario: a project with neither a description file nor config has no description
    Given a project root containing repository "solo.git"
    And "solo.git" has no description file
    When I read the metadata of "solo.git"
    Then there is no description

  # --- owner ---

  Scenario: an owner is read from gitweb config
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "owner" set to "Ada Lovelace"
    When I read the metadata of "solo.git"
    Then the owner is "Ada Lovelace"

  Scenario: a project with no configured owner has no owner
    Given a project root containing repository "solo.git"
    When I read the metadata of "solo.git"
    Then there is no owner

  # --- category: file and config ---

  Scenario: a category is read from the category file
    Given a project root containing repository "solo.git"
    And "solo.git" has the category file "tools"
    When I read the metadata of "solo.git"
    Then the category is "tools"

  Scenario: a category falls back to gitweb config when there is no file
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "category" set to "libraries"
    When I read the metadata of "solo.git"
    Then the category is "libraries"

  # --- clone URLs: zero / one / many ---

  Scenario: a project with no clone URLs has none
    Given a project root containing repository "solo.git"
    When I read the metadata of "solo.git"
    Then there are 0 clone URLs

  Scenario: a single clone URL comes from gitweb config
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "url" set to "git://example.org/solo.git"
    When I read the metadata of "solo.git"
    Then there is 1 clone URL
    And the clone URLs include "git://example.org/solo.git"

  Scenario: several clone URLs come from the cloneurl file in order
    Given a project root containing repository "solo.git"
    And "solo.git" has the clone URLs "git://example.org/solo.git" and "https://example.org/solo.git"
    When I read the metadata of "solo.git"
    Then there are 2 clone URLs
    And the clone URLs include "git://example.org/solo.git"
    And the clone URLs include "https://example.org/solo.git"

  # --- the missing and unsafe edges, shared with open() ---

  Scenario: reading metadata for a name that does not exist fails as not found
    Given an empty project root
    When I read the metadata of "ghost.git"
    Then reading metadata fails as not found

  Scenario: a path-traversal name is rejected as invalid
    Given an empty project root
    When I read the metadata of "../etc"
    Then reading metadata fails as invalid
