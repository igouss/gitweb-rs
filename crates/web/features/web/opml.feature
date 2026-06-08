Feature: Serving the OPML project outline (opml action)

  gitweb's git_opml serves a text/xml "opml.xml": an OPML 1.0 outline with one
  entry per project that has a HEAD, each linking to the project's feed and
  summary page. The action takes no project. An empty root is die_error(404).

  Scenario: the opml outline lists the head-bearing projects
    Given a repository "fh.git" with a file history
    And the opml action is served
    When I GET "/?a=opml"
    Then the response status is 200
    And the response content type is "text/xml; charset=utf-8"
    And the response is offered inline as "opml.xml"
    And the response body contains "<opml version="1.0">"
    And the response body contains "<outline type="rss" text="fh.git" title="fh.git" xmlUrl="http://localhost/?p=fh.git;a=rss" htmlUrl="http://localhost/?p=fh.git;a=summary"/>"

  Scenario: an empty project root has no outline
    Given the opml action is served
    When I GET "/?a=opml"
    Then the response status is 404
