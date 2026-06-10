Feature: Advertising feed auto-discovery links in the page head (print_feed_meta)

  gitweb prints <link rel="alternate"> feed metadata in the <head> of every
  200 OK HTML page (print_header_links). A project page advertises four feeds —
  RSS and Atom, each plain and "(no merges)" — titled "<project> - <feed title>
  - <FORMAT> feed", with the feed title naming the branch when the page views
  one. The projects list advertises its plain-text index and OPML feed instead.

  Scenario: a project page advertises its RSS and Atom feeds in the head
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body contains "<link rel="alternate" type="application/rss+xml" title="repo.git - log - RSS feed" href="/?p=repo.git&amp;a=rss">"
    And the response body contains "<link rel="alternate" type="application/rss+xml" title="repo.git - log - RSS feed (no merges)" href="/?p=repo.git&amp;a=rss&amp;opt=--no-merges">"
    And the response body contains "<link rel="alternate" type="application/atom+xml" title="repo.git - log - Atom feed" href="/?p=repo.git&amp;a=atom">"
    And the response body contains "<link rel="alternate" type="application/atom+xml" title="repo.git - log - Atom feed (no merges)" href="/?p=repo.git&amp;a=atom&amp;opt=--no-merges">"

  Scenario: a page viewing a branch ref scopes its feed to that branch
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary&hb=refs/heads/topic"
    Then the response status is 200
    And the response body contains "title="repo.git - log of topic - RSS feed" href="/?p=repo.git&amp;a=rss&amp;h=refs%2Fheads%2Ftopic">"

  Scenario: a page scoped to a file scopes its feed to that file's history
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary&f=src/foo.c"
    Then the response status is 200
    And the response body contains "title="repo.git - history of src/foo.c - RSS feed" href="/?p=repo.git&amp;a=rss&amp;f=src%2Ffoo.c">"

  Scenario: the projects list advertises its index and OPML feeds
    Given a project root containing repository "alpha.git"
    And the project-list landing page is served
    When I GET "/"
    Then the response status is 200
    And the response body contains "<link rel="alternate" type="text/plain; charset=utf-8" title="Untitled Git projects list" href="/?a=project_index">"
    And the response body contains "<link rel="alternate" type="text/x-opml" title="Untitled Git projects feeds" href="/?a=opml">"

  Scenario: a project-filtered projects list scopes its index and OPML feeds to the subdirectory
    Given a project root containing repository "group/alpha.git"
    And the root also contains repository "other/beta.git"
    And the project-list landing page is served
    When I GET "/?pf=group"
    Then the response status is 200
    And the response body contains "<link rel="alternate" type="text/plain; charset=utf-8" title="Untitled Git projects list" href="/?a=project_index&amp;pf=group">"
    And the response body contains "<link rel="alternate" type="text/x-opml" title="Untitled Git projects feeds" href="/?a=opml&amp;pf=group">"
