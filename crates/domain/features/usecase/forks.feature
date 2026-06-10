Feature: Assembling the forks page
  The forks use case (gitweb's git_forks) lists a project's forks. Forks of
  `repo.git` live in the `repo/` directory, so the page scopes a project listing
  to that subdirectory — the project path with one trailing `.git` removed — and
  dies "404 No forks found" when the project has none. It reuses the project-list
  assembly, so with the forks feature on, forks-of-forks fold under their parent.

  Scenario: the forks of a project are listed
    Given the forks feature is enabled
    And the store has project "repo.git"
    And the store has project "repo/one.git"
    And the store has project "repo/two.git"
    And the store has project "other.git"
    When I assemble the forks of "repo.git"
    Then the listed projects are "repo/one.git, repo/two.git"

  Scenario: a project with no forks is the not-found error
    Given the forks feature is enabled
    And the store has project "repo.git"
    When I assemble the forks of "repo.git"
    Then assembling fails as not found
    And the not-found message is "No forks found"

  Scenario: a forks subdirectory holding nothing is the not-found error
    Given the forks feature is enabled
    And the store has project "repo.git"
    And the store has project "other/one.git"
    When I assemble the forks of "repo.git"
    Then assembling fails as not found
    And the not-found message is "No forks found"

  Scenario: forks of forks fold under their parent on the forks page
    Given the forks feature is enabled
    And the store has project "repo.git"
    And the store has project "repo/one.git"
    And the store has project "repo/one/deep.git"
    When I assemble the forks of "repo.git"
    Then the listed projects are "repo/one.git"
    And the project "repo/one.git" reports 1 fork
