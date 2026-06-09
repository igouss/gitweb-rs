Feature: Serving the machine-readable project index (project_index action)

  gitweb's git_project_index serves a text/plain "index.aux": one CGI-quoted
  "path owner" line per project under the root, offered inline under that
  filename. The action takes no project. An empty root is die_error(404).

  Scenario: the project index lists the projects as text
    Given a repository "fh.git" with a file history
    And the project index is served
    When I GET "/?a=project_index"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response is offered inline as "index.aux"
    And the response body contains "fh.git"

  Scenario: an empty project root has no index
    Given the project index is served
    When I GET "/?a=project_index"
    Then the response status is 404

  Scenario: a project filter scopes the index to its subdirectory
    Given a project root containing repository "group/alpha.git"
    And the root also contains repository "other/beta.git"
    And the project index is served
    When I GET "/?a=project_index&pf=group"
    Then the response status is 200
    And the response body contains "group/alpha.git"
    And the response body does not contain "other/beta.git"
