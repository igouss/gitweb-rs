Feature: RSS and Atom feeds match gitweb's reference output

  The syndication feeds are format-stable, so they are verified by golden
  differential conformance: the bytes our use case + web view-model + render
  serializers produce over the corpus must equal the reference captured once from
  the original gitweb.perl — body, media type, and Last-Modified alike.

  Scenario: the RSS feed is byte-for-byte gitweb's
    Given the parity corpus
    When I serve the "rss" feed
    Then the feed body matches gitweb's reference output
    And the feed media type matches gitweb's
    And the feed last-modified matches gitweb's

  Scenario: the Atom feed is byte-for-byte gitweb's
    Given the parity corpus
    When I serve the "atom" feed
    Then the feed body matches gitweb's reference output
    And the feed media type matches gitweb's
    And the feed last-modified matches gitweb's
