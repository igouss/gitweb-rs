Feature: Serving the projects-list landing page
  gitweb's default page (git_project_list) lists every discoverable project under
  the root. The assembled router resolves a bare request to the project_list
  action and serves the page as HTML; an empty root is gitweb's
  "404 No projects found"; an unknown sort order is "400 Unknown order parameter".
  This drives the real gix ProjectStore over a project root laid down on disk.

  Scenario: an empty project root reports no projects found
    Given an empty project root
    And the project-list landing page is served
    When I GET "/"
    Then the response status is 404
    And the response body contains "No projects found"

  Scenario: the landing page lists the discovered projects as HTML
    Given a project root containing repository "alpha.git"
    And the root also contains repository "beta.git"
    And the project-list landing page is served
    When I GET "/"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "alpha.git"
    And the response body contains "beta.git"
    And the response body contains "Last Change"

  Scenario: the landing page dates a project by its extra branch directory activity
    Given a repository "extra.git" with an unborn HEAD
    And the repository "extra.git" has a ref "refs/sandbox/wip" committed at 1700000000
    And the project-list landing page is served with extra branch refs "sandbox"
    When I GET "/"
    Then the response status is 200
    And the response body contains "extra.git"
    And the response body does not contain "No commits"

  Scenario: without the feature an extra-directory-only project shows no last change
    Given a repository "extra.git" with an unborn HEAD
    And the repository "extra.git" has a ref "refs/sandbox/wip" committed at 1700000000
    And the project-list landing page is served
    When I GET "/"
    Then the response status is 200
    And the response body contains "extra.git"
    And the response body contains "No commits"

  Scenario: an unknown sort order is rejected
    Given a project root containing repository "alpha.git"
    And the project-list landing page is served
    When I GET "/?o=sideways"
    Then the response status is 400

  Scenario: a project filter scopes the landing page to its subdirectory
    Given a project root containing repository "group/alpha.git"
    And the root also contains repository "other/beta.git"
    And the project-list landing page is served
    When I GET "/?pf=group"
    Then the response status is 200
    And the response body contains "group/alpha.git"
    And the response body does not contain "other/beta.git"

  Scenario: a project filter matching nothing is no projects found
    Given a project root containing repository "group/alpha.git"
    And the project-list landing page is served
    When I GET "/?pf=elsewhere"
    Then the response status is 404
    And the response body contains "No projects found"

  Scenario: an unsafe project filter is rejected
    Given a project root containing repository "group/alpha.git"
    And the project-list landing page is served
    When I GET "/?pf=../etc"
    Then the response status is 404
    And the response body contains "Invalid project_filter parameter"
