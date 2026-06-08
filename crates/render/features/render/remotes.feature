Feature: Rendering the remotes page
  The remotes page (modernized git_remotes) shows, per configured remote, a small
  URL table (gitweb's "URL" / "Fetch URL" / "Push URL", or "No remote URL") over
  the remote's tracking branches rendered by the same heads table the heads page
  uses. In the all-remotes view each remote's name links to its single-remote
  view; the single-remote view shows one block with no name link. URLs and labels
  are built by the web boundary, so this layer only lays them out and escapes them.

  Scenario: the page shows its title
    Given a remotes page titled "proj remotes"
    When I render the remotes page
    Then the result contains "proj remotes"

  Scenario: an all-remotes block links its name to the single-remote view
    Given a remotes page titled "proj remotes"
    And a remote block "origin" linking to "/r/remotes/origin"
    When I render the remotes page
    Then the result contains ">origin<"
    And the result contains "/r/remotes/origin"

  Scenario: a single-remote block shows no name link
    Given a remotes page titled "origin remote for proj"
    And a remote block "origin" with no link
    When I render the remotes page
    Then the result does not contain "/r/remotes/origin"

  Scenario: a combined URL line is labelled and shows its value
    Given a remotes page titled "proj remotes"
    And a remote block "origin" with no link
    And the remote block has a URL line labelled "URL" of "git://h/r"
    When I render the remotes page
    Then the result contains "URL"
    And the result contains "git://h/r"

  Scenario: distinct fetch and push URL lines are labelled
    Given a remotes page titled "proj remotes"
    And a remote block "origin" with no link
    And the remote block has a URL line labelled "Fetch URL" of "git://h/r"
    And the remote block has a URL line labelled "Push URL" of "ssh://h/r"
    When I render the remotes page
    Then the result contains "Fetch URL"
    And the result contains "Push URL"

  Scenario: a missing URL shows the placeholder
    Given a remotes page titled "proj remotes"
    And a remote block "origin" with no link
    And the remote block has an unlabelled URL line of "No remote URL"
    When I render the remotes page
    Then the result contains "No remote URL"

  Scenario: a remote's tracking branch renders in the heads table
    Given a remotes page titled "proj remotes"
    And a remote block "origin" with no link
    And the remote block tracks branch "main" at "/r/shortlog"
    When I render the remotes page
    Then the result contains ">main<"
    And the result contains "/r/shortlog"
