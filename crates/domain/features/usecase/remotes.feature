Feature: Viewing a project's remotes (remotes action)
  The remotes use case (gitweb's git_remotes) lists the configured remotes, each
  with its fetch/push URL lines and its remote-tracking branches under
  refs/remotes/<name>/ — enriched exactly as the heads listing does, but shown
  without the <name>/ prefix. The all-remotes view shows every remote; the
  single-remote view shows just the named one. The whole view is gated behind the
  remote_heads feature: disabled, it is forbidden. With no remotes configured, or
  a named remote that is not configured, it is not found.

  Background:
    Given the current time is 1000000
    And the remote_heads feature is enabled

  # --- the feature gate (gitweb's die_error(403)) ---

  Scenario: the view is forbidden when the remote_heads feature is disabled
    Given the remote_heads feature is disabled
    And a remote "origin" fetching from "git://h/r"
    When I assemble the remotes
    Then assembling the remotes fails as forbidden

  # --- discovery (zero / one / many) ---

  Scenario: a project with no remotes configured is not found
    When I assemble the remotes
    Then assembling the remotes fails as not found

  Scenario: a single configured remote is shown
    Given a remote "origin" fetching from "git://h/r"
    When I assemble the remotes
    Then the shown remotes are "origin"

  Scenario: several remotes are all shown
    Given a remote "origin" fetching from "git://h/r"
    And a remote "upstream" fetching from "git://h/u"
    When I assemble the remotes
    Then the shown remotes are "origin, upstream"

  # --- the URL block, per remote (the rule itself is covered in remote.feature) ---

  Scenario: a remote's block carries its URL lines
    Given a remote "origin" fetching from "git://h/r" pushing to "ssh://h/r"
    When I assemble the remotes
    Then the remote "origin" URL lines are "fetch git://h/r, push ssh://h/r"

  # --- tracking branches (zero / one / many) ---

  Scenario: a remote with no tracking branches lists none
    Given a remote "origin" fetching from "git://h/r"
    When I assemble the remotes
    Then the remote "origin" tracks no branches

  Scenario: a tracking branch is listed under its name, the remote stripped
    Given a remote "origin" fetching from "git://h/r"
    And the remote "origin" tracks branch "main" committed at 900000
    When I assemble the remotes
    Then the remote "origin" tracks "main"

  Scenario: tracking branches are ordered newest commit first
    Given a remote "origin" fetching from "git://h/r"
    And the remote "origin" tracks branch "main" committed at 100
    And the remote "origin" tracks branch "topic" committed at 200
    When I assemble the remotes
    Then the remote "origin" tracks "topic, main"

  Scenario: a tracking branch shows its tip age
    Given a remote "origin" fetching from "git://h/r"
    And the remote "origin" tracks branch "main" committed at 999400
    When I assemble the remotes
    Then the remote "origin" tracking branch "main" shows the age "10 min ago"

  Scenario: a tracking branch at HEAD's commit is marked current
    Given the repository HEAD is at commit "x"
    And a remote "origin" fetching from "git://h/r"
    And the remote "origin" tracks branch "main" at commit "x"
    When I assemble the remotes
    Then the remote "origin" tracking branch "main" is current

  # --- the single-remote view (hash=<name>) ---

  Scenario: the single-remote view shows just the named remote
    Given a remote "origin" fetching from "git://h/r"
    And a remote "upstream" fetching from "git://h/u"
    When I assemble the remote "origin"
    Then the shown remotes are "origin"
    And the remotes view is the single-remote view

  Scenario: the single-remote view for a remote that is not configured is not found
    Given a remote "origin" fetching from "git://h/r"
    When I assemble the remote "ghost"
    Then assembling the remotes fails as not found
