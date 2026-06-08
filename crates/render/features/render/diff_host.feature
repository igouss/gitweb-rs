Feature: The diff-viewer host page (modernized git_commitdiff HTML view)
  gitweb's git_commitdiff colourises the diff server-side with format_diff_line.
  We modernize that: the host page ships a #diff-root container carrying the URL
  of a clean unified diff, and boots a client module that fetches it and renders
  it with the vendored @pierre/diffs viewer. The empty-diff and rendered-diff
  states are the client's job (documented manual checks); this layer only places
  the container, the boot module, a loading placeholder, and a no-JavaScript
  fallback to the raw diff. URLs are built by the web boundary, so this layer
  takes finished hrefs: the diff URL the viewer fetches and the module src.

  Scenario: the diff-root container carries the URL the viewer fetches
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    When I render the diff host page
    Then the result contains "id="diff-root""
    And the result contains "data-diff-url="/?p=t.git;a=commitdiff_plain;h=deadbeef""

  Scenario: the page boots the client viewer module
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    When I render the diff host page
    Then the result contains "<script type="module" src="/static/diff-viewer.js">"

  Scenario: a loading placeholder is shown before the client renders
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    When I render the diff host page
    Then the result contains "Loading diff"

  Scenario: a no-JavaScript fallback links to the raw diff
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    When I render the diff host page
    Then the result contains "<noscript>"
    And the result contains "View the raw unified diff"
    And the result contains "href="/?p=t.git;a=commitdiff_plain;h=deadbeef""

  Scenario: the commit subject is the page title
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    And the diff host title is "Fix the scheduler race"
    When I render the diff host page
    Then the result contains "Fix the scheduler race"

  Scenario: the format affordances are shown
    Given a diff host fetching "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the viewer module is served at "/static/diff-viewer.js"
    And the diff host offers format "raw" at "/?p=t.git;a=commitdiff_plain;h=deadbeef"
    And the diff host offers format "patch" at "/?p=t.git;a=patch;h=deadbeef"
    When I render the diff host page
    Then the result contains ">raw</a>"
    And the result contains ">patch</a>"

  Scenario: a single-file diff shows the file path
    Given a diff host fetching "/?p=t.git;a=blobdiff;f=src/main.rs"
    And the viewer module is served at "/static/diff-viewer.js"
    And the diff host is scoped to file "src/main.rs"
    When I render the diff host page
    Then the result contains "src/main.rs"
