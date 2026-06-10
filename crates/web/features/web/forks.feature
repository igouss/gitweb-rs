Feature: Serving the forks view and folding forks on the landing page
  gitweb's forks feature (off by default) folds a project's forks under it on the
  landing list — showing a leading "+" linking to the forks action — and serves
  the forks action (git_forks), which lists the forks of one project under a
  "$project forks" header. Forks of repo.git live in the sibling repo/ directory.
  This drives the real gix ProjectStore over a project root laid down on disk.

  Scenario: the landing page folds a fork under its parent
    Given a project root containing repository "repo.git"
    And the root also contains repository "repo/one.git"
    And the forks feature is served
    When I GET "/"
    Then the response status is 200
    And the response body contains "repo.git"
    And the response body contains "a=forks"
    And the response body does not contain "one.git"

  Scenario: the forks action lists a project's forks
    Given a project root containing repository "repo.git"
    And the root also contains repository "repo/one.git"
    And the root also contains repository "repo/two.git"
    And the forks feature is served
    When I GET "/?p=repo.git&a=forks"
    Then the response status is 200
    And the response body contains "repo.git forks"
    And the response body contains "repo/one.git"
    And the response body contains "repo/two.git"

  Scenario: a project with no forks reports no forks found
    Given a project root containing repository "repo.git"
    And the forks feature is served
    When I GET "/?p=repo.git&a=forks"
    Then the response status is 404
    And the response body contains "No forks found"

  Scenario: the forks list excludes the project itself and string-prefix siblings
    Given a project root containing repository "repo.git"
    And the root also contains repository "repo/one.git"
    And the root also contains repository "repobar.git"
    And the forks feature is served
    When I GET "/?p=repo.git&a=forks"
    Then the response status is 200
    And the response body contains "repo/one.git"
    And the response body does not contain "repobar.git"
