Feature: Reading configured remotes (remotes port)
  The gix adapter reads a repository's configured remotes the way gitweb's
  git_get_remotes_list does — each remote's name and the fetch/push URLs
  git remote -v reports, in name order — and exposes the remote-tracking branches
  as ordinary references under refs/remotes/<name>/. A remote with only a url
  reports the same fetch and push URL (git's push falls back to the fetch url); a
  remote with a distinct pushurl reports both; a remote with only a pushurl
  reports no fetch url.

  Background:
    Given a repository with remotes

  Scenario: a repository with no remotes lists none
    Given a repository with no remotes
    When I read the remotes
    Then no remotes are listed

  Scenario: the configured remotes are listed in name order
    When I read the remotes
    Then the listed remotes are "mirror, origin, pushonly"

  Scenario: a remote with only a url reports it as both fetch and push
    When I read the remotes
    Then the remote "origin" fetch URL is "git://h/origin"
    And the remote "origin" push URL is "git://h/origin"

  Scenario: a remote with a distinct pushurl reports both URLs
    When I read the remotes
    Then the remote "mirror" fetch URL is "git://h/mirror"
    And the remote "mirror" push URL is "ssh://h/mirror"

  Scenario: a remote with only a pushurl reports no fetch URL
    When I read the remotes
    Then the remote "pushonly" has no fetch URL
    And the remote "pushonly" push URL is "ssh://h/pushonly"

  Scenario: a remote's tracking branches are references under refs/remotes
    When I list references under "refs/remotes/origin/"
    Then 2 remote references are listed
