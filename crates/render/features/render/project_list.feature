Feature: Rendering the projects-list table
  gitweb renders the projects list as a table with sortable column headers and
  one row per project. The column the list is sorted by is plain text; the others
  link to re-sort. Each row links the project name and description to its summary,
  shows the owner and the last-change age (coloured by recency, or "No commits"
  when the project has none), and carries quick links to summary, shortlog, log
  and tree. Git-derived text is escaped by default. URLs come finished from the
  web boundary, so this view only lays them out.

  # --- sortable headers (active is plain text, others link) ---

  Scenario: the active sort column is plain text and the others link to re-sort
    Given a listed project "git.git" at "/git"
    When I render the project list sorted by "project"
    Then the result contains "<th class="sorted">Project</th>"
    And the result contains "<a class="sort" href="/?o=descr">Description</a>"

  # --- a populated row ---

  Scenario: a project row shows its name, metadata and age
    Given a listed project "git.git" at "/git" described "The stupid content tracker" owned by "Junio" aged 600
    When I render the project list sorted by "project"
    Then the result contains "git.git"
    And the result contains "The stupid content tracker"
    And the result contains "Junio"
    And the result contains "10 min ago"
    And the result contains "age-fresh"

  Scenario: a row links to summary, shortlog, log and tree
    Given a listed project "git.git" at "/git"
    When I render the project list sorted by "project"
    Then the result contains ">summary</a>"
    And the result contains ">shortlog</a>"
    And the result contains ">log</a>"
    And the result contains ">tree</a>"

  # --- the no-commits edge ---

  Scenario: a project with no commits shows "No commits"
    Given a listed project "void.git" at "/void" with no commits
    When I render the project list sorted by "project"
    Then the result contains "No commits"
    And the result contains "age-unknown"

  # --- escaping ---

  Scenario: a project name with HTML metacharacters is escaped
    Given a listed project "a<b>.git" at "/x"
    When I render the project list sorted by "project"
    Then the result contains "a&lt;b&gt;.git"
    And the result does not contain "<b>.git"

  # --- zero rows still renders the table chrome ---

  Scenario: an empty list still renders the table and its headers
    When I render the project list sorted by "project"
    Then the result contains "<table class="project-list">"
    And the result contains "Last Change"

  # --- the forks feature: leading '+' column linking to the forks view ---

  Scenario: a project with forks shows a linked '+' and a forks quick link
    Given a listed project "repo.git" at "/repo" with 2 forks
    When I render the project list sorted by "project"
    Then the result contains "<a href="/repo/forks" title="2 forks">+</a>"
    And the result contains "<a href="/repo/forks">forks</a>"

  Scenario: with the fork column on, a fork-less project has an empty cell and no forks link
    Given a listed project "solo.git" at "/solo" with 0 forks
    When I render the project list sorted by "project"
    Then the result contains "<th class="forks"></th>"
    And the result does not contain ">+</a>"
    And the result does not contain ">forks</a>"
