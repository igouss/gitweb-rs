Feature: RSS and Atom feed serialization (gitweb git_feed)

  Feeds are format-stable, so the serializer reproduces gitweb's XML to the byte.
  These scenarios pin the structure, the date forms, and the per-site escaping;
  the parity crate's golden test proves byte-exactness against real gitweb.

  Scenario: the RSS channel header and metadata
    Given a feed with one commit
    When I render the feed as RSS
    Then the body contains "<?xml version="1.0" encoding="utf-8"?>"
    And the body contains "<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">"
    And the body contains "<title>Untitled - repo log</title>"
    And the body contains "<description>desc</description>"
    And the body contains "<managingEditor>Ada Lovelace</managingEditor>"
    And the body contains "<url>static/git-logo.png</url>"
    And the body contains "<pubDate>Tue, 14 Nov 2023 22:13:20 +0000</pubDate>"
    And the body contains "<generator>gitweb v.gitweb-x/2.0</generator>"

  Scenario: the RSS item escapes text and wraps the body in CDATA
    Given a feed with one commit
    When I render the feed as RSS
    Then the body contains "<author>Ada Lovelace &lt;ada@example.com&gt;</author>"
    And the body contains "<guid isPermaLink="true">http://h/?p=repo;a=commitdiff;h=abc</guid>"
    And the body contains "<content:encoded><![CDATA["
    And the body contains "fix &lt;stuff&gt;"
    And the body contains "<li>[<a href="http://h/?p=repo;a=blobdiff;f=a%26b.txt" title="diff">D</a><a href="http://h/?p=repo;a=history;f=a%26b.txt" title="history">H</a>] a&amp;b.txt</li>"
    And the body contains "</ul>]]>"

  Scenario: the Atom feed header and self link
    Given a feed with one commit
    When I render the feed as Atom
    Then the body contains "<feed xmlns="http://www.w3.org/2005/Atom">"
    And the body contains "<subtitle>desc</subtitle>"
    And the body contains "<link rel="self" type="application/atom+xml" href="http://h?p=repo;a=atom" />"
    And the body contains "<id>http://h/?p=repo</id>"
    And the body contains "<icon>static/git-favicon.png</icon>"
    And the body contains "<logo>static/git-logo.png</logo>"
    And the body contains "<updated>2023-11-14T22:13:20Z</updated>"
    And the body contains "<generator version='gitweb-x/2.0'>gitweb</generator>"

  Scenario: the Atom entry carries author, contributor, and xhtml content
    Given a feed with one commit
    When I render the feed as Atom
    Then the body contains "<title type="html">fix &lt;stuff&gt;</title>"
    And the body contains "  <email>ada@example.com</email>"
    And the body contains "<content type="xhtml" xml:base="http://h/">"
    And the body contains "</div>"

  Scenario: an empty feed keeps a valid Atom date and no items
    Given an empty feed
    When I render the feed as Atom
    Then the body contains "<updated>1970-01-01T00:00:00Z</updated>"
    And the body does not contain "<entry>"

  Scenario: an empty RSS feed omits the channel date but stays well-formed
    Given an empty feed
    When I render the feed as RSS
    Then the body contains "</channel>"
    And the body does not contain "<pubDate>"
    And the body does not contain "<item>"
