Feature: Feed auto-discovery info (gitweb get_feed_info)

  The pure classification behind a page's <head> <link rel="alternate"> feed
  metadata. From the page's action, the ref it is viewing (hash base or hash),
  and an optional file, gitweb's get_feed_info decides the feed's descriptive
  title ("log", "log of <branch>", "history of <file>"), whether the page
  scopes to a branch (matching the ref against the configured branch
  directories, honouring extra-branch-refs), and which file it narrows to.
  Some actions (tags, heads, forks, tag, search) and the actionless page get
  the generic project log with no scope.

  Background:
    Given the branch ref directories "heads"

  Scenario: a plain page is a log
    Given the feed page action "summary"
    When I classify the feed info
    Then the feed-info title is "log"
    And the feed-info has no branch hash
    And the feed-info has no file

  Scenario: an excluded action drops even a named file
    Given the feed page action "tags"
    And the feed page file "src/foo.c"
    When I classify the feed info
    Then the feed-info title is "log"
    And the feed-info has no branch hash
    And the feed-info has no file

  Scenario: an absent action is a plain log
    Given the feed page action ""
    When I classify the feed info
    Then the feed-info title is "log"
    And the feed-info has no branch hash

  Scenario: a branch named in the hash base makes it a log of that branch
    Given the feed page action "log"
    And the feed page hash base "refs/heads/topic"
    When I classify the feed info
    Then the feed-info title is "log of topic"
    And the feed-info branch hash is "refs/heads/topic"

  Scenario: the hash parameter is matched when the hash base is not
    Given the feed page action "shortlog"
    And the feed page hash "refs/heads/topic"
    When I classify the feed info
    Then the feed-info title is "log of topic"
    And the feed-info branch hash is "refs/heads/topic"

  Scenario: a short ref name is not a branch match
    Given the feed page action "log"
    And the feed page hash base "master"
    When I classify the feed info
    Then the feed-info title is "log"
    And the feed-info has no branch hash

  Scenario: a named file alone makes it a history
    Given the feed page action "history"
    And the feed page file "src/foo.c"
    When I classify the feed info
    Then the feed-info title is "history of src/foo.c"
    And the feed-info has no branch hash
    And the feed-info file is "src/foo.c"

  Scenario: a file on the tree action gets a trailing slash
    Given the feed page action "tree"
    And the feed page file "docs"
    When I classify the feed info
    Then the feed-info title is "history of docs/"
    And the feed-info file is "docs"

  Scenario: a file and a branch together name both
    Given the feed page action "history"
    And the feed page hash base "refs/heads/topic"
    And the feed page file "src/foo.c"
    When I classify the feed info
    Then the feed-info title is "history of src/foo.c on 'topic'"
    And the feed-info branch hash is "refs/heads/topic"
    And the feed-info file is "src/foo.c"

  Scenario: an extra branch directory classifies its refs as branches
    Given the branch ref directories "heads remotes"
    And the feed page action "log"
    And the feed page hash base "refs/remotes/origin/topic"
    When I classify the feed info
    Then the feed-info title is "log of origin/topic"
    And the feed-info branch hash is "refs/remotes/origin/topic"
