Feature: Serving a project's summary page (summary action)
  gitweb's git_summary is the default landing page: it opens the project, shows
  its metadata (description, owner, last change, clone URLs), an optional raw
  README, then the recent shortlog and the tags and heads — each capped with a
  "..." link past 16. An unborn repository shows its metadata without sections
  and without failing, and a request for a project that does not exist is
  gitweb's die_error(404).

  Scenario: the summary page shows the metadata and the sections
    Given a project root containing repository "repo.git"
    And "repo.git" has the description file "A neat project"
    And the repository "repo.git" has branch "topic" committed at 2000
    And the repository "repo.git" has an annotated tag "v1" of a commit at 1500 with subject "Release"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "A neat project"
    And the response body contains "topic"
    And the response body contains "v1"
    And the response body contains "shortlog"
    And the response body contains "tags"
    And the response body contains "heads"

  Scenario: a present README is injected verbatim
    Given a project root containing repository "repo.git"
    And "repo.git" has a README of "<em>read me</em>"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body contains "<em>read me</em>"
    And the response body contains "readme"

  Scenario: an absent README leaves no readme block
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body does not contain "readme-body"

  Scenario: an empty README is treated as absent
    Given a project root containing repository "repo.git"
    And "repo.git" has an empty README
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body does not contain "readme-body"

  Scenario: XSS prevention suppresses the README
    Given a project root containing repository "repo.git"
    And "repo.git" has a README of "<em>read me</em>"
    And the summary action is served with XSS prevention
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body does not contain "read me"

  Scenario: the "..." truncation link appears past 16 branches
    Given a repository "big.git" with an unborn HEAD
    And the repository "big.git" has 17 branches
    And the summary action is served
    When I GET "/?p=big.git&a=summary"
    Then the response status is 200
    And the response body contains "..."

  Scenario: an unborn repository serves its summary without sections
    Given a repository "void.git" with an unborn HEAD
    And the summary action is served
    When I GET "/?p=void.git&a=summary"
    Then the response status is 200
    And the response body contains "description"

  Scenario: summary for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=ghost.git&a=summary"
    Then the response status is 404
