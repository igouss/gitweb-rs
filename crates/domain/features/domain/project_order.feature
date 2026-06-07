Feature: Ordering the projects list
  gitweb validates the projects-list `order` parameter against a fixed set and
  otherwise dies "400 Unknown order parameter" (git_project_list); the chosen key
  then drives sort_projects_list. Sorting by project, description or owner is a
  plain string comparison, where a missing field compares as empty; sorting by
  age shows the most recently changed project first and sinks projects with no
  commits to the bottom. "none" is a valid order that leaves the list untouched.

  # --- parsing the order parameter ---

  Scenario Outline: a recognized order parses to its own key
    When I parse the project order "<token>"
    Then the parsed order is "<token>"

    Examples:
      | token   |
      | none    |
      | project |
      | descr   |
      | owner   |
      | age     |

  Scenario: an unrecognized order is rejected
    When I parse the project order "sideways"
    Then parsing the order is rejected as invalid

  # --- sorting: zero / one / many ---

  Scenario: ordering an empty list yields an empty list
    When I order the listing by "project"
    Then the listing order is ""

  Scenario: ordering a single project yields just that project
    Given the listing has a project "only.git"
    When I order the listing by "project"
    Then the listing order is "only.git"

  Scenario: ordering by project sorts the paths
    Given the listing has a project "zlib.git"
    And the listing has a project "acl.git"
    And the listing has a project "make.git"
    When I order the listing by "project"
    Then the listing order is "acl.git, make.git, zlib.git"

  # --- sorting: the string-key edge (a missing field compares as empty) ---

  Scenario: ordering by owner sorts by owner, a missing owner sorting first
    Given the listing has a project "b.git" owned by "Zed"
    And the listing has a project "a.git" owned by "Ann"
    And the listing has a project "c.git" with no owner
    When I order the listing by "owner"
    Then the listing order is "c.git, a.git, b.git"

  Scenario: ordering by description sorts by the full description
    Given the listing has a project "two.git" described as "Beta tool"
    And the listing has a project "one.git" described as "Alpha tool"
    When I order the listing by "descr"
    Then the listing order is "one.git, two.git"

  # --- sorting: the age edge (recent first, no-commit projects last) ---

  Scenario: ordering by age shows the most recent first and no-commit projects last
    Given the listing has a project "old.git" last changed at 1000
    And the listing has a project "fresh.git" last changed at 3000
    And the listing has a project "void.git" with no commits
    When I order the listing by "age"
    Then the listing order is "fresh.git, old.git, void.git"

  # --- "none" is a no-op ---

  Scenario: "none" leaves the list in its original order
    Given the listing has a project "zlib.git"
    And the listing has a project "acl.git"
    When I order the listing by "none"
    Then the listing order is "zlib.git, acl.git"
