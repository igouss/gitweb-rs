Feature: Serving a project's remotes (remotes action)
  gitweb's git_remotes page lists the configured remotes — each with its
  fetch/push URLs and its remote-tracking branches — and serves it as an HTML
  page. The whole view is gated behind the remote_heads feature: disabled, it is
  a 403. With no remotes configured, or a single-remote request for a remote that
  is not configured, it is gitweb's die_error(404). A request for a project that
  does not exist is also a 404.

  Scenario: the all-remotes page lists the configured remotes and their URLs
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has branch "main" committed at 1000
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the repository "repo.git" has a remote "upstream" fetching "git://h/upstream"
    And the remotes action is served
    When I GET "/?p=repo.git&a=remotes"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "origin"
    And the response body contains "upstream"
    And the response body contains "git://h/origin"
    And the response body contains "repo.git remotes"

  Scenario: a remote's tracking branches are listed
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the repository "repo.git" remote "origin" tracks "trunk" committed at 1000
    And the remotes action is served
    When I GET "/?p=repo.git&a=remotes"
    Then the response status is 200
    And the response body contains "trunk"

  Scenario: the single-remote view shows just that remote
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the repository "repo.git" has a remote "upstream" fetching "git://h/upstream"
    And the remotes action is served
    When I GET "/?p=repo.git&a=remotes&h=origin"
    Then the response status is 200
    And the response body contains "origin remote for repo.git"
    And the response body does not contain "upstream"

  Scenario: a single-remote request for a remote that is not configured is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the remotes action is served
    When I GET "/?p=repo.git&a=remotes&h=ghost"
    Then the response status is 404

  Scenario: a project with no remotes configured is not found
    Given a repository "repo.git" with an unborn HEAD
    And the remotes action is served
    When I GET "/?p=repo.git&a=remotes"
    Then the response status is 404

  Scenario: the remotes view is forbidden when the remote_heads feature is disabled
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the remotes action is served with the remote_heads feature disabled
    When I GET "/?p=repo.git&a=remotes"
    Then the response status is 403

  Scenario: remotes for a project that does not exist is not found
    Given a repository "repo.git" with an unborn HEAD
    And the repository "repo.git" has a remote "origin" fetching "git://h/origin"
    And the remotes action is served
    When I GET "/?p=ghost.git&a=remotes"
    Then the response status is 404
