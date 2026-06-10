Feature: Feed auto-discovery links (gitweb print_feed_meta)

  Laying out the <link rel="alternate"> feed metadata a page advertises in its
  <head>. A project page advertises four links — RSS and Atom, each in a plain
  and a "(no merges)" variant — titled "<project> - <feed title> - <FORMAT>
  feed". The projects list instead advertises its plain-text project index and
  its OPML feed, titled with the site name. The descriptive feed title and the
  URLs are decided upstream; this lays out the links, their wording, their
  order, and their media types.

  Scenario: a project page advertises four RSS and Atom links
    When I assemble the project feed links for "proj.git" titled "log of topic"
    Then the feed links number 4
    And feed link 0 is titled "proj.git - log of topic - RSS feed" linking "/rss" of type "application/rss+xml"
    And feed link 1 is titled "proj.git - log of topic - RSS feed (no merges)" linking "/rss-nm" of type "application/rss+xml"
    And feed link 2 is titled "proj.git - log of topic - Atom feed" linking "/atom" of type "application/atom+xml"
    And feed link 3 is titled "proj.git - log of topic - Atom feed (no merges)" linking "/atom-nm" of type "application/atom+xml"

  Scenario: the projects list advertises its index and OPML feeds
    When I assemble the project-list feed links for site "Example Git"
    Then the feed links number 2
    And feed link 0 is titled "Example Git projects list" linking "/index" of type "text/plain; charset=utf-8"
    And feed link 1 is titled "Example Git projects feeds" linking "/opml" of type "text/x-opml"
