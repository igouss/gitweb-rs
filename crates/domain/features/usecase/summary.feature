Feature: Assembling the summary page (summary action)
  The summary use case (gitweb's git_summary) composes the project's landing
  page from the metadata the boundary resolved and the refs and history the
  repository holds: a description (or gitweb's "none"), the owner unless display
  is omitted, HEAD's commit time as the last change, the project's clone URLs
  (or a base-URL fallback), an optional raw README, then the recent shortlog,
  the tags, and the heads. An unborn repository shows its metadata but no last
  change and an empty shortlog, without failing.

  Background:
    Given the current time is 1000000

  # --- description (gitweb's "none" placeholder) ---

  Scenario: a project with no description shows the "none" placeholder
    Given the repository HEAD is the unborn branch "main"
    When I assemble the summary
    Then the summary description is "none"

  Scenario: a described project shows its description
    Given the repository HEAD is the unborn branch "main"
    And the project is described as "A neat little project"
    When I assemble the summary
    Then the summary description is "A neat little project"

  # --- owner (suppressed by omit_owner) ---

  Scenario: an owned project shows its owner
    Given the repository HEAD is the unborn branch "main"
    And the project is owned by "Ada Lovelace"
    When I assemble the summary
    Then the summary shows the owner "Ada Lovelace"

  Scenario: the owner is suppressed when owner display is omitted
    Given the repository HEAD is the unborn branch "main"
    And the project is owned by "Ada Lovelace"
    And owner display is omitted
    When I assemble the summary
    Then the summary shows no owner

  # --- last change: HEAD's committer time, absolute ---

  Scenario: the last change is HEAD's commit date
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 1577880000
    When I assemble the summary
    Then the summary last change date is "2020-01-01"

  Scenario: an unborn repository shows no last change
    Given the repository HEAD is the unborn branch "main"
    When I assemble the summary
    Then the summary shows no last change

  # --- clone URLs: own list, base fallback, or none ---

  Scenario: a project advertises its own clone URLs
    Given the repository HEAD is the unborn branch "main"
    And the project has clone url "git://example.org/repo.git"
    When I assemble the summary
    Then the summary clone urls are "git://example.org/repo.git"

  Scenario: clone URLs fall back to the base URL and project name
    Given the repository HEAD is the unborn branch "main"
    And the project is named "repo.git"
    And the git base URL is "git://example.org"
    When I assemble the summary
    Then the summary clone urls are "git://example.org/repo.git"

  Scenario: a project with neither clone URLs nor base URLs advertises none
    Given the repository HEAD is the unborn branch "main"
    When I assemble the summary
    Then the summary advertises no clone urls

  # --- README: raw sink, suppressed by prevent_xss ---

  Scenario: a project README is included verbatim
    Given the repository HEAD is the unborn branch "main"
    And the project README is "<h1>Hi</h1>"
    When I assemble the summary
    Then the summary README is "<h1>Hi</h1>"

  Scenario: the README is suppressed under XSS prevention
    Given the repository HEAD is the unborn branch "main"
    And the project README is "<h1>Hi</h1>"
    And XSS prevention is on
    When I assemble the summary
    Then the summary includes no README

  # --- the three sections (reused shortlog / tags / heads) ---

  Scenario: recent commits appear in the shortlog section
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 900000 by "Ada" titled "First"
    And a commit "c2" at epoch 800000 by "Babbage" titled "Second"
    When I assemble the summary
    Then the summary shortlog lists "c1, c2"

  Scenario: tags appear in the tags section, newest first
    Given the repository HEAD is the unborn branch "main"
    And a lightweight tag "v1" on a commit at 900000
    And an annotated tag "v2" of a commit tagged at 800000 with subject "Release"
    When I assemble the summary
    Then the summary lists tags "v1, v2"
    And the summary tags section is not truncated

  Scenario: heads appear in the heads section, newest first
    Given the repository HEAD is branch "main"
    And the repository has branch "main" committed at 900000
    And the repository has branch "topic" committed at 800000
    When I assemble the summary
    Then the summary lists heads "main, topic"
    And the summary heads section is not truncated

  Scenario: an unborn repository has an empty shortlog and does not fail
    Given the repository HEAD is the unborn branch "main"
    When I assemble the summary
    Then the summary shortlog is empty
