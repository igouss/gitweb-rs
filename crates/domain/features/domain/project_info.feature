Feature: Per-project metadata for the projects list
  gitweb fills each project on the list with a description, owner, category and
  clone URLs from the filesystem and git config. The display derivations are
  pure: the short description is the long one chopped to a fixed width (gitweb's
  chop_str with a 5-character slack), and a project with no category falls back
  to the configured default category.

  # --- short description (chop_str at width 25, slack 5) ---

  Scenario: a project with no description has an empty short description
    Given a project "void.git" with no description
    When I shorten its description to width 25
    Then the short description is ""

  Scenario: a description within the width is shown in full
    Given a project "tiny.git" described as "A short tagline"
    When I shorten its description to width 25
    Then the short description is "A short tagline"

  Scenario: an over-long description is chopped at a word boundary with a filler
    Given a project "wordy.git" described as "The quick brown fox jumps over the lazy dog"
    When I shorten its description to width 25
    Then the short description is "The quick brown fox jumps... "

  # --- category fallback ---

  Scenario: a project with no category falls back to the default
    Given a project "loose.git" with no category
    When I read its category with default "misc"
    Then the category is "misc"

  Scenario: a project keeps its own category over the default
    Given a project "filed.git" in category "tools"
    When I read its category with default "misc"
    Then the category is "tools"

  # --- grouping by category (zero / one / many) ---

  Scenario: grouping an empty list yields no categories
    Given no projects to group
    When I group by category with default "misc"
    Then 0 categories result

  Scenario: grouping one categorized project yields its single category
    Given a project "a.git" in category "tools" to group
    When I group by category with default "misc"
    Then 1 category results
    And the category "tools" holds 1 project
    And the category "tools" holds "a.git"

  Scenario: many projects bucket into their categories, uncategorized to the default
    Given a project "a.git" in category "tools" to group
    And a project "b.git" in category "tools" to group
    And a project "c.git" with no category to group
    When I group by category with default "misc"
    Then 2 categories result
    And the category "tools" holds 2 projects
    And the category "tools" holds "a.git"
    And the category "tools" holds "b.git"
    And the category "misc" holds 1 project
    And the category "misc" holds "c.git"
