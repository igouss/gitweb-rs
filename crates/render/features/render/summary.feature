Feature: Rendering the summary page
  The summary page (modernized git_summary) stacks a metadata block (description,
  owner, last change, clone URLs), an optional raw README, and the shortlog /
  tags / heads sections under headers that link to their full actions. The
  section tables are the same ones the standalone pages render, so this layer
  adds only the metadata block, the raw-README sink, and the per-section headers.
  URLs are built by the web boundary, so this view takes finished hrefs.

  Scenario: the metadata block shows the description
    Given a summary described as "A neat project"
    When I render the summary
    Then the result contains "description"
    And the result contains "A neat project"

  Scenario: the metadata block shows the owner when present
    Given a summary described as "x" owned by "Ada Lovelace"
    When I render the summary
    Then the result contains "owner"
    And the result contains "Ada Lovelace"

  Scenario: the owner row is absent when there is no owner
    Given a summary described as "x"
    When I render the summary
    Then the result does not contain "metadata_owner"

  Scenario: the metadata block shows the last change when present
    Given a summary with a last change
    When I render the summary
    Then the result contains "last change"
    And the result contains "commit-date"

  Scenario: a clone URL is labelled and shown
    Given a summary with clone url "git://example.org/repo.git"
    When I render the summary
    Then the result contains "URL"
    And the result contains "git://example.org/repo.git"

  Scenario: the README is injected as raw HTML
    Given a summary with README "<em>hello</em>"
    When I render the summary
    Then the result contains "<em>hello</em>"
    And the result contains "readme"

  Scenario: there is no README block when there is no README
    Given a summary described as "x"
    When I render the summary
    Then the result does not contain "readme-body"

  Scenario: the shortlog section links to the full action
    Given a summary with a shortlog section at "/r/shortlog"
    When I render the summary
    Then the result contains "/r/shortlog"
    And the result contains "shortlog"

  Scenario: the tags section links to the full action
    Given a summary with a tags section at "/r/tags"
    When I render the summary
    Then the result contains "/r/tags"

  Scenario: the heads section links to the full action
    Given a summary with a heads section at "/r/heads"
    When I render the summary
    Then the result contains "/r/heads"
