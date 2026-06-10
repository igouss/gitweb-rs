Feature: The page footer links a project's feeds and the projects list's index (git_footer_html)

  gitweb's git_footer_html closes every HTML page. In project context it links
  the page's RSS and Atom feeds (visible anchors, titled "<feed title> <FORMAT>
  feed" from get_feed_info, scoped to the branch a page views); on the projects
  list it links the OPML feed and the plain-text project index. The links use
  the modernized href dialect (& between params, maud-escaped), the same as
  every other link on the page.

  Scenario: a project page footer links its RSS and Atom feeds
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary"
    Then the response status is 200
    And the response body contains "<a class="feed" href="/?p=repo.git&amp;a=rss" title="log RSS feed">RSS</a>"
    And the response body contains "<a class="feed" href="/?p=repo.git&amp;a=atom" title="log Atom feed">Atom</a>"

  Scenario: a branch-scoped project page footer names the branch in the feed title
    Given a project root containing repository "repo.git"
    And the summary action is served
    When I GET "/?p=repo.git&a=summary&hb=refs/heads/topic"
    Then the response status is 200
    And the response body contains "<a class="feed" href="/?p=repo.git&amp;a=rss&amp;h=refs%2Fheads%2Ftopic" title="log of topic RSS feed">RSS</a>"
    And the response body contains "<a class="feed" href="/?p=repo.git&amp;a=atom&amp;h=refs%2Fheads%2Ftopic" title="log of topic Atom feed">Atom</a>"

  Scenario: the projects-list footer links the OPML feed and the plain-text index
    Given a project root containing repository "alpha.git"
    And the project-list landing page is served
    When I GET "/"
    Then the response status is 200
    And the response body contains "<a class="feed" href="/?a=opml" title="OPML">OPML</a>"
    And the response body contains "<a class="feed" href="/?a=project_index" title="TXT">TXT</a>"
