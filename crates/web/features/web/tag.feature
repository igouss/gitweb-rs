Feature: Serving a single annotated tag (tag action)
  gitweb's git_tag opens the project, resolves the request hash, and renders the
  tag object: its name, the tagged object linked by its kind, the tagger
  authorship, and the message body. A hash that names anything other than a tag
  object — a commit, or a lightweight tag's object — is die_error(404, "Unknown
  tag object"); a project that does not exist, or a missing hash, is also a 404.

  Scenario: the tag page shows the tag's object, tagger and message
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the tag action is served
    When I GET "/?p=repo.git&a=tag&h=v1.0"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains ">v1.0<"
    And the response body contains "Release 1.0"
    And the response body contains "Ada Lovelace"
    And the response body contains ">commit<"

  Scenario: a lightweight tag's object is not a tag object
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a lightweight tag "rc1" of a commit at 1000
    And the tag action is served
    When I GET "/?p=repo.git&a=tag&h=rc1"
    Then the response status is 404
    And the response body contains "Unknown tag object"

  Scenario: a plain commit is not a tag object
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the tag action is served
    When I GET "/?p=repo.git&a=tag&h=main"
    Then the response status is 404
    And the response body contains "Unknown tag object"

  Scenario: a missing hash is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the tag action is served
    When I GET "/?p=repo.git&a=tag&h=ghost"
    Then the response status is 404

  Scenario: the tag action for a project that does not exist is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has an annotated tag "v1.0" of a commit at 1000 with subject "Release 1.0"
    And the tag action is served
    When I GET "/?p=ghost.git&a=tag&h=v1.0"
    Then the response status is 404
