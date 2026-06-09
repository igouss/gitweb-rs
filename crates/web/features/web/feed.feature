Feature: Serving RSS and Atom feeds (rss / atom actions)

  gitweb's git_feed serves the recent log of a branch (HEAD by default) as a
  syndication feed: the right feed media type, a Last-Modified from the newest
  commit, and one item/entry per commit with absolute links. A request for a
  project that does not exist is die_error(404).

  Scenario: the RSS feed serves the recent log
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=rss"
    Then the response status is 200
    And the response content type is "application/rss+xml; charset=utf-8"
    And the response last-modified is "Wed, 15 Nov 2023 22:13:20 +0000"
    And the response body contains "<rss version="2.0""
    And the response body contains "<title>Untitled Git - fh.git/rss log</title>"
    And the response body contains "<generator>gitweb v.gitweb-test/1</generator>"
    And the response body contains "edit file.txt"
    And the response body contains "add file.txt"
    And the response body contains "http://localhost/?p=fh.git;a=commitdiff;h="

  Scenario: the Atom feed serves the recent log
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=atom"
    Then the response status is 200
    And the response content type is "application/atom+xml; charset=utf-8"
    And the response last-modified is "Wed, 15 Nov 2023 22:13:20 +0000"
    And the response body contains "<feed xmlns="http://www.w3.org/2005/Atom">"
    And the response body contains "<link rel="self" type="application/atom+xml" href="http://localhost?p=fh.git;a=atom" />"
    And the response body contains "<entry>"
    And the response body contains "edit file.txt"

  Scenario: a feed for a project that does not exist is not found
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=ghost.git&a=rss"
    Then the response status is 404

  Scenario: a feed whose cached copy is current returns 304 with no body
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=rss" if not modified since "Wed, 15 Nov 2023 22:13:20 +0000"
    Then the response status is 304
    And the response last-modified is "Wed, 15 Nov 2023 22:13:20 +0000"
    And the response body is empty

  Scenario: a feed whose cached copy is stale is served in full
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=rss" if not modified since "Wed, 15 Nov 2023 21:13:20 +0000"
    Then the response status is 200
    And the response body contains "<rss version="2.0""

  Scenario: a HEAD feed request returns the headers but no body
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I HEAD "/?p=fh.git&a=rss"
    Then the response status is 200
    And the response content type is "application/rss+xml; charset=utf-8"
    And the response last-modified is "Wed, 15 Nov 2023 22:13:20 +0000"
    And the response body is empty

  Scenario: a reader preferring text/xml receives the feed as text/xml
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=rss" accepting "text/xml"
    Then the response status is 200
    And the response content type is "text/xml; charset=utf-8"
    And the response body contains "<rss version="2.0""

  Scenario: a reader accepting the feed's own type keeps it
    Given a repository "fh.git" with a file history
    And the feed actions are served
    When I GET "/?p=fh.git&a=atom" accepting "application/atom+xml"
    Then the response status is 200
    And the response content type is "application/atom+xml; charset=utf-8"
