Feature: Rendering the pickaxe-results table
  The pickaxe page (modernized git_search_changes) lists one commit per row: the
  date, the author, and the subject (linking to the commit), with — under the
  subject — each file whose occurrence count the commit changed, its path in a
  <span class="match"> linking to the file's blob, then a "commit | tree" link
  cell. Deletions are dropped upstream (no blob to link), and gitweb prints no "no
  matches" note for pickaxe, so an empty result is simply an empty table. URLs are
  built by the web boundary, so this layer takes finished hrefs and only decides
  layout and escaping.

  Scenario: a commit row links its subject to the commit and lists a changed file
    Given a pickaxe commit "Add the recipe" by "Ada" at commit "/r/commit/x" tree "/r/tree/x"
    And the pickaxe commit changed file "recipe.txt" at "/r/blob/r"
    When I render the pickaxe page
    Then the result contains "/r/commit/x"
    And the result contains "Add the recipe"
    And the result contains "/r/tree/x"
    And the result contains "<span class="match">recipe.txt</span>"
    And the result contains "/r/blob/r"
    And the result contains "commit"
    And the result contains "tree"

  Scenario: a commit that changed several files lists each with its own blob link
    Given a pickaxe commit "Spread the string" by "Ada" at commit "/r/commit/x" tree "/r/tree/x"
    And the pickaxe commit changed file "a.txt" at "/r/blob/a"
    And the pickaxe commit changed file "b.txt" at "/r/blob/b"
    When I render the pickaxe page
    Then the result contains "<span class="match">a.txt</span>"
    And the result contains "/r/blob/a"
    And the result contains "<span class="match">b.txt</span>"
    And the result contains "/r/blob/b"

  Scenario: a commit with no listed files still shows its row but no match span
    Given a pickaxe commit "Delete the string" by "Ada" at commit "/r/commit/x" tree "/r/tree/x"
    When I render the pickaxe page
    Then the result contains "Delete the string"
    And the result contains "/r/commit/x"
    And the result does not contain "<span class="match">"

  Scenario: an empty result is an empty table with no note (gitweb prints none)
    When I render the pickaxe page
    Then the result contains "pickaxe"
    And the result does not contain "No matches found"

  Scenario: the date cell carries the swapped tooltip and the author its full name
    Given a pickaxe commit "Edit" by "Ada Lovelace" short "Ada Lov..." at commit "/r/commit/x" tree "/r/tree/x"
    And the pickaxe commit is dated "2026-01-01" tooltip "3 days ago"
    When I render the pickaxe page
    Then the result contains "2026-01-01"
    And the result contains "3 days ago"
    And the result contains "Ada Lovelace"
    And the result contains "Ada Lov..."
