Feature: Assembling the search-help page
  The search-help use case reads the site's grep and pickaxe feature gates and
  documents exactly the search types the user could run. It touches no
  repository — the page is pure help prose — so it always succeeds. The grep and
  pickaxe types are enabled by default, so the out-of-the-box page documents
  every type; disabling a feature drops its type from the page.

  Scenario: the default configuration documents every search type
    When I assemble the search help
    Then the documented topics are "commit, grep, author, committer, pickaxe"

  Scenario: disabling grep drops the grep type
    Given the grep feature is disabled
    When I assemble the search help
    Then the documented topics are "commit, author, committer, pickaxe"

  Scenario: disabling pickaxe drops the pickaxe type
    Given the pickaxe feature is disabled
    When I assemble the search help
    Then the documented topics are "commit, grep, author, committer"

  Scenario: disabling both leaves only the always-available types
    Given the grep feature is disabled
    And the pickaxe feature is disabled
    When I assemble the search help
    Then the documented topics are "commit, author, committer"
