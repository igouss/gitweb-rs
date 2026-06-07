Feature: Repository export visibility
  Only repositories allowed by the server's export rules are served, mirroring
  gitweb's check_head_link, check_export_ok and the $strict_export gate
  (is_valid_project). A repository is visible only when it has a linked HEAD and
  satisfies every gate the server has enabled — export marker, auth hook, and
  strict projects-list membership. A gate that is not enabled imposes no
  constraint.

  Scenario: A linked repository is visible under a permissive policy
    Given a repository whose HEAD is linked
    And a permissive export policy
    When I evaluate visibility
    Then the repository is visible

  Scenario: A repository without a linked HEAD is never visible
    Given a repository whose HEAD is not linked
    And a permissive export policy
    When I evaluate visibility
    Then the repository is hidden

  Scenario: Requiring an export marker hides a repository that lacks it
    Given a repository whose HEAD is linked
    And an export marker is required
    And the export marker is absent
    When I evaluate visibility
    Then the repository is hidden

  Scenario: Requiring an export marker admits a repository that has it
    Given a repository whose HEAD is linked
    And an export marker is required
    And the export marker is present
    When I evaluate visibility
    Then the repository is visible

  Scenario: Strict export hides a repository absent from the projects list
    Given a repository whose HEAD is linked
    And strict export is enabled
    And the repository is not in the projects list
    When I evaluate visibility
    Then the repository is hidden

  Scenario: Strict export admits a repository present in the projects list
    Given a repository whose HEAD is linked
    And strict export is enabled
    And the repository is in the projects list
    When I evaluate visibility
    Then the repository is visible

  Scenario: An auth hook can deny an otherwise-linked repository
    Given a repository whose HEAD is linked
    And an auth hook is configured
    And the auth hook denies the repository
    When I evaluate visibility
    Then the repository is hidden

  Scenario: An auth hook can admit a linked repository
    Given a repository whose HEAD is linked
    And an auth hook is configured
    And the auth hook allows the repository
    When I evaluate visibility
    Then the repository is visible

  Scenario: With every gate enabled, all must pass for visibility
    Given a repository whose HEAD is linked
    And an export marker is required
    And the export marker is present
    And an auth hook is configured
    And the auth hook allows the repository
    And strict export is enabled
    And the repository is in the projects list
    When I evaluate visibility
    Then the repository is visible

  Scenario: With every gate enabled, one failing gate hides the repository
    Given a repository whose HEAD is linked
    And an export marker is required
    And the export marker is absent
    And an auth hook is configured
    And the auth hook allows the repository
    And strict export is enabled
    And the repository is in the projects list
    When I evaluate visibility
    Then the repository is hidden
