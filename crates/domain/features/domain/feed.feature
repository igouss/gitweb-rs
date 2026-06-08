Feature: Syndication feed rules (gitweb git_feed)

  The pure rules behind RSS/Atom feeds: the feed title and its "feed type" word,
  the window of commits a feed shows (the newest 20 plus anything within 48
  hours), and a commit message rendered as the feed's <pre> body lines.

  # --- feed title and feed type --------------------------------------------

  Scenario: a plain branch feed is a log
    When I build the feed title for action "rss" with no branch and no file
    Then the feed title is "Untitled Git - repo.git/rss log"

  Scenario: a named branch makes it a branch log
    When I build the feed title for action "rss" with branch "topic" and no file
    Then the feed title is "Untitled Git - repo.git/rss - 'topic' branch log"

  Scenario: a named file alone makes it a history
    When I build the feed title for action "atom" with no branch and file "src/foo.c"
    Then the feed title is "Untitled Git - repo.git/atom - src/foo.c history"

  Scenario: a branch and a file together make it a history
    When I build the feed title for action "atom" with branch "topic" and file "src/foo.c"
    Then the feed title is "Untitled Git - repo.git/atom - 'topic' :: src/foo.c history"

  # --- feed window: newest 20, plus anything within 48 hours ----------------

  Scenario: an empty history keeps nothing
    Given the feed clock is 1000000000
    When I window the feed
    Then the feed keeps 0 commits

  Scenario: a short history keeps every commit, however old
    Given the feed clock is 1000000000
    And 3 feed commits aged 1000 hours
    When I window the feed
    Then the feed keeps 3 commits

  Scenario: beyond the newest 20, stale commits are dropped
    Given the feed clock is 1000000000
    And 25 feed commits aged 1000 hours
    When I window the feed
    Then the feed keeps 20 commits

  Scenario: beyond the newest 20, fresh commits are still shown
    Given the feed clock is 1000000000
    And 22 feed commits aged 1 hours
    And 3 feed commits aged 1000 hours
    When I window the feed
    Then the feed keeps 22 commits

  # --- comment lines: <pre> body --------------------------------------------

  Scenario: an empty message has no comment lines
    When I extract the feed comment from ""
    Then the feed comment lines are ""

  Scenario: a one-line message
    When I extract the feed comment from "fix the bug"
    Then the feed comment lines are "fix the bug"

  Scenario: trailing blank lines are dropped and a four-space indent stripped
    When I extract the feed comment from "    title\n\nbody\n\n\n"
    Then the feed comment lines are "title||body"
