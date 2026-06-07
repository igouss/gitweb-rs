Feature: Serving a project's tags (tags action)
  gitweb's git_tags page opens the project, lists its tags newest-first with their
  ages, links each tag to the object it names, shows the annotation subject and a
  tag selflink for annotated tags, and serves it as an HTML page. A project with
  no tags lists nothing without failing, and a request for a project that does not
  exist is gitweb's die_error(404).

  Scenario: the tags page lists the project's tags
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the repository "repo.git" has a lightweight tag "nightly" of a commit at 2000
    And the tags action is served
    When I GET "/?p=repo.git&a=tags"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "v1.0"
    And the response body contains "nightly"
    And the response body contains "Release 1.0"
    And the response body contains "Tags"

  Scenario: an annotated tag offers a tag selflink
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the tags action is served
    When I GET "/?p=repo.git&a=tags"
    Then the response status is 200
    And the response body contains ">tag<"

  Scenario: a repository with no tags serves an empty tags page
    Given a repository "void.git" with an unborn HEAD
    And the tags action is served
    When I GET "/?p=void.git&a=tags"
    Then the response status is 200
    And the response body contains "No tags."

  Scenario: tags for a project that does not exist is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the tags action is served
    When I GET "/?p=ghost.git&a=tags"
    Then the response status is 404
