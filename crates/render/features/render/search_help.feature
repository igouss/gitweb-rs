Feature: Rendering the search-help page (git_search_help)
  gitweb's search-help page explains how patterns are matched and documents each
  search type in a definition list. The intro prose is always shown; each type's
  entry appears only when its topic is in the list, so a site without the grep or
  pickaxe features renders a page without those entries.

  Scenario: the page explains pattern matching and documents every type
    When I render the search help page documenting every type
    Then the body contains "Pattern"
    And the body contains "regular expression"
    And the body contains "The commit messages and authorship information will be scanned"
    And the body contains "All files in the currently selected tree"
    And the body contains "Name and e-mail of the change author"
    And the body contains "Name and e-mail of the committer"
    And the body contains "appear or disappear from any file"

  Scenario: a page without the gated types omits their entries
    When I render the search help page documenting only the always-available types
    Then the body contains "The commit messages and authorship information will be scanned"
    And the body does not contain "All files in the currently selected tree"
    And the body does not contain "appear or disappear from any file"
