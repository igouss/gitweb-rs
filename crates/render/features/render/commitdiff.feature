Feature: Rendering the commitdiff host page (modernized git_commitdiff HTML view)
  gitweb's git_commitdiff renders a commit's authorship and message, the
  changed-files table, and then colourises the diff itself. We modernize that:
  the page keeps everything around the diff — the navigation with its alternate
  formats and parent/merge context, the author and committer rows, the message
  body, and the changed-files table — but replaces the server-coloured patchset
  with a #diff-root container the client viewer fills from a clean unified diff.
  URLs are built by the web boundary, so this layer takes finished hrefs.

  Scenario: a single-parent commitdiff shows authorship, message, files and the diff root
    When I render a single-parent commitdiff page
    Then the result contains "Rework the engine"
    And the result contains "Ada Lovelace"
    And the result contains "Linus Torvalds"
    And the result contains "Detailed body line."
    And the result contains "README"
    And the result contains "id="diff-root""
    And the result contains "data-diff-url="/r/diff/c0ffee""
    And the result contains "(parent: "
    And the result contains ">raw</a>"

  Scenario: a root commitdiff shows the initial marker and no parent context
    When I render a root commitdiff page
    Then the result contains "(initial)"
    And the result does not contain "(parent:"
    And the result does not contain "(merge:"

  Scenario: a merge commitdiff lists every parent
    When I render a merge commitdiff page
    Then the result contains "(merge: "
    And the result contains "/r/commit/par1"
    And the result contains "/r/commit/par2"

  Scenario: a commitdiff between two commits names the chosen parent
    When I render a commitdiff page from a chosen parent
    Then the result contains "(from parent 2: "
    And the result contains "/r/commit/par2"

  Scenario: the patch format affordance is offered
    When I render a single-parent commitdiff page
    Then the result contains "/r/patch/c0ffee"

  Scenario: the no-JavaScript fallback links to the raw diff
    When I render a single-parent commitdiff page
    Then the result contains "<noscript>"
    And the result contains "View the raw unified diff"
