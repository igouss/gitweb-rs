Feature: Rendering the heads table
  The heads table (modernized git_heads_body) shows one row per branch: its
  last-change age coloured by recency (or "unknown" for a tip with no commit
  time), the branch name linking to its shortlog, and per-branch shortlog / log /
  tree links. The branch whose tip is HEAD's commit is marked current. URLs are
  built by the web boundary, so this layer takes finished hrefs and only decides
  layout, escaping, and the recency/current CSS hooks.

  Scenario: a branch row shows its age, name link, and per-branch links
    Given a head "main" at "/r/shortlog" aged 600
    When I render the heads table
    Then the result contains "10 min ago"
    And the result contains "/r/shortlog"
    And the result contains "/r/shortlog/log"
    And the result contains "/r/shortlog/tree"
    And the result contains ">main<"

  Scenario: the age cell carries a recency class
    Given a head "main" at "/x" aged 600
    When I render the heads table
    Then the result contains "age-fresh"

  Scenario: the current branch is marked
    Given a current head "main" at "/x" aged 600
    When I render the heads table
    Then the result contains "current"

  Scenario: a non-current branch is not marked
    Given a head "main" at "/x" aged 600
    When I render the heads table
    Then the result does not contain "current"

  Scenario: a branch tip with no commit time shows an unknown age
    Given a head "main" at "/x" with an unknown age
    When I render the heads table
    Then the result contains "unknown"

  Scenario: every branch gets its own row
    Given a head "main" at "/main" aged 600
    And a head "topic" at "/topic" aged 600
    When I render the heads table
    Then the result contains ">main<"
    And the result contains ">topic<"

  Scenario: a capped list gets a trailing "more" row linking to the full heads
    Given a head "main" at "/main" aged 600
    And the heads offer more at "/r/heads" labelled "..."
    When I render the heads table
    Then the result contains "/r/heads"
    And the result contains "..."

  Scenario: an uncapped list has no "more" row
    Given a head "main" at "/main" aged 600
    When I render the heads table
    Then the result does not contain "more"

  Scenario: the heads page body wraps the table in the page chrome
    Given a head "main" at "/r/shortlog" aged 600
    When I render the heads page
    Then the result contains "class="page-header""
    And the result contains "Heads"
    And the result contains ">main<"
    And the result contains "class="page-footer""
