Feature: OPML project outline serialization (gitweb git_opml)

  The OPML outline is format-stable, so the serializer reproduces gitweb's XML to
  the byte: the <opml> wrapper, a <title> of "<site name> OPML Export", and one
  <outline type="rss" …/> per project inside the "git RSS feeds" group. The
  parity crate's golden test proves byte-exactness against real gitweb.

  Scenario: a single project renders the full outline
    Given an opml outline for site "Untitled Git"
    And an opml project "repo.git" with feed "http://localhost/?p=repo.git;a=rss" and summary "http://localhost/?p=repo.git;a=summary"
    When I render the opml outline
    Then the rendered body is the newline-ended lines:
      """
      <?xml version="1.0" encoding="utf-8"?>
      <opml version="1.0">
      <head>
        <title>Untitled Git OPML Export</title>
      </head>
      <body>
      <outline text="git RSS feeds">
      <outline type="rss" text="repo.git" title="repo.git" xmlUrl="http://localhost/?p=repo.git;a=rss" htmlUrl="http://localhost/?p=repo.git;a=summary"/>
      </outline>
      </body>
      </opml>
      """

  Scenario: an outline with no head-bearing projects keeps the wrapper but lists nothing
    Given an opml outline for site "Untitled Git"
    When I render the opml outline
    Then the rendered body is the newline-ended lines:
      """
      <?xml version="1.0" encoding="utf-8"?>
      <opml version="1.0">
      <head>
        <title>Untitled Git OPML Export</title>
      </head>
      <body>
      <outline text="git RSS feeds">
      </outline>
      </body>
      </opml>
      """

  Scenario: the site name and project path are escaped
    Given an opml outline for site "A & B"
    And an opml project "a<b" with feed "u1" and summary "u2"
    When I render the opml outline
    Then the body contains "<title>A &amp; B OPML Export</title>"
    And the body contains "text="a&lt;b" title="a&lt;b""

  Scenario: a project filter titles the scoped subdirectory
    Given an opml outline for site "Untitled Git"
    And the opml outline is filtered to subdirectory "group/team"
    When I render the opml outline
    Then the body contains "<title>Untitled Git OPML Export within subdirectory group/team</title>"

  Scenario: the filter subdirectory is escaped in the title
    Given an opml outline for site "Untitled Git"
    And the opml outline is filtered to subdirectory "a&b"
    When I render the opml outline
    Then the body contains "OPML Export within subdirectory a&amp;b</title>"
