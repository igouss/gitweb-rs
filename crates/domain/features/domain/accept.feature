Feature: Feed content-type negotiation (Accept)

  gitweb's git_feed downgrades a feed from application/{rss,atom}+xml to text/xml
  when the reader's Accept scores text/xml strictly higher than the feed's own
  media type. No Accept header keeps the feed type, and so do equal scores
  (including a bare wildcard) since the comparison is strict.

  Scenario: no Accept header keeps the feed type
    Given an RSS feed
    When I negotiate with no Accept header
    Then the feed type is kept

  Scenario: an Accept of text/xml prefers the downgrade
    Given an RSS feed
    When I negotiate with Accept "text/xml"
    Then the feed type is downgraded to text/xml

  Scenario: an Accept of the feed's own type keeps it
    Given an RSS feed
    When I negotiate with Accept "application/rss+xml"
    Then the feed type is kept

  Scenario: a bare wildcard Accept keeps the feed type
    Given an RSS feed
    When I negotiate with Accept "*/*"
    Then the feed type is kept

  Scenario: a higher-q text/xml prefers the downgrade
    Given an RSS feed
    When I negotiate with Accept "application/rss+xml;q=0.5, text/xml;q=0.9"
    Then the feed type is downgraded to text/xml

  Scenario: a higher-q Atom type keeps the feed
    Given an Atom feed
    When I negotiate with Accept "text/xml;q=0.5, application/atom+xml;q=0.9"
    Then the feed type is kept

  Scenario: text/xml reached only through a wildcard does not outscore the feed type
    Given an RSS feed
    When I negotiate with Accept "application/rss+xml, */*"
    Then the feed type is kept
