Feature: A configured remote's URL lines (remotes view)
  gitweb's git_remote_block renders a remote's URLs as a small table whose rows
  depend on which of the fetch and push URLs are set. When both are set and equal
  it collapses to one combined "URL" row; when they differ it shows a "Fetch URL"
  row and a "Push URL" row; with only one of them it shows just that row; with
  neither it shows a single "No remote URL" placeholder. The display labels are
  the boundary's — this is the pure rule that decides which lines exist.

  Scenario: a shared fetch and push URL collapses to one combined line
    Given a remote "origin" fetching from "git://h/r" pushing to "git://h/r"
    When I read the remote's URL lines
    Then the URL lines are "combined git://h/r"

  Scenario: distinct fetch and push URLs show as two lines
    Given a remote "origin" fetching from "git://h/r" pushing to "ssh://h/r"
    When I read the remote's URL lines
    Then the URL lines are "fetch git://h/r, push ssh://h/r"

  Scenario: only a fetch URL shows just the fetch line
    Given a remote "origin" fetching from "git://h/r"
    When I read the remote's URL lines
    Then the URL lines are "fetch git://h/r"

  Scenario: only a push URL shows just the push line
    Given a remote "origin" pushing to "ssh://h/r"
    When I read the remote's URL lines
    Then the URL lines are "push ssh://h/r"

  Scenario: no URL at all shows the placeholder line
    Given a remote "origin" with no URLs
    When I read the remote's URL lines
    Then the URL lines are "missing"
