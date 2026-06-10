Feature: Rendering the forks page
  gitweb renders a project's forks with the same project-list table the landing
  page uses, under a "$project forks" header and the per-project page navigation.
  This view adds only the page furniture — the breadcrumb header, the action bar,
  and the title — around the reused table. The fork column is always on here, so
  a fork that has its own forks shows the linked '+'.

  Scenario: the forks page shows the project title and the forks table
    Given a breadcrumb link "git" to "/"
    And a current breadcrumb "forks"
    And a navigation link "summary" to "/?p=repo.git;a=summary"
    And a listed project "repo/one.git" at "/?p=repo/one.git;a=summary" with 0 forks
    When I render the forks page for "repo.git" sorted by "project"
    Then the result contains "repo.git forks"
    And the result contains "<table class="project-list">"
    And the result contains ">summary</a>"
    And the result contains "repo/one.git"

  Scenario: a fork that itself has forks shows the linked '+' on the forks page
    Given a breadcrumb link "git" to "/"
    And a listed project "repo/one.git" at "/one" with 3 forks
    When I render the forks page for "repo.git" sorted by "project"
    Then the result contains "<a href="/one/forks" title="3 forks">+</a>"
