Feature: Serving the diff-viewer client assets
  The diff host page (modernized git_commitdiff) boots a client module that
  fetches a commit's clean unified diff and renders it with the vendored pierre
  diffs viewer. That boot module is baked into the binary and served at a stable
  URL with a JavaScript MIME type, the way the stylesheet and favicon are — no
  filesystem, nothing to go missing at runtime. The ~11 MB vendored viewer bundle
  it imports is a separate, git-ignored, built-on-demand asset served by the
  composition root; its absence is documented to degrade gracefully, not crash
  the server.

  Scenario: the boot module is served as an ES module with a JavaScript MIME type
    Given an empty project root
    When I GET "/static/diff-viewer.js"
    Then the response status is 200
    And the response content type is "text/javascript; charset=utf-8"

  Scenario: the boot module reads the diff URL from the host container and renders it
    Given an empty project root
    When I GET "/static/diff-viewer.js"
    Then the response body contains "data-diff-url"
    And the response body contains "parsePatchFiles"
    And the response body contains "/static/vendor/pierre/index.js"
