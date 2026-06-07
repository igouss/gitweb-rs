Feature: Commit derivations
  A commit's display fields are pure derivations over its structured data,
  matching the merge test and title computation gitweb does in
  parse_commit_text.

  A commit is a merge when it has more than one parent; its title is the first
  non-empty message line, chopped to 80 characters (50 for the short title),
  falling back to "(no commit message)".

  Scenario: A root commit with no parents is not a merge
    Given a commit with 0 parents
    When I check whether it is a merge
    Then it is not a merge

  Scenario: An ordinary commit with one parent is not a merge
    Given a commit with 1 parents
    When I check whether it is a merge
    Then it is not a merge

  Scenario: A commit with two parents is a merge
    Given a commit with 2 parents
    When I check whether it is a merge
    Then it is a merge

  Scenario: An octopus merge with three parents is a merge
    Given a commit with 3 parents
    When I check whether it is a merge
    Then it is a merge

  Scenario: The title is the first message line
    Given a commit with the message "Fix the off-by-one in the scheduler"
    When I read its title
    And I read its short title
    Then the title is "Fix the off-by-one in the scheduler"
    And the short title is "Fix the off-by-one in the scheduler"

  Scenario: A leading blank line is skipped for the title
    Given a commit whose message starts with a blank line then "Real subject"
    When I read its title
    Then the title is "Real subject"

  Scenario: An empty message falls back to the placeholder
    Given a commit with an empty message
    When I read its title
    And I read its short title
    Then the title is "(no commit message)"
    And the short title is "(no commit message)"

  Scenario: A message of only blank lines falls back to the placeholder
    Given a commit whose message is two blank lines
    When I read its title
    Then the title is "(no commit message)"

  Scenario: The title is bounded at 80 characters and the short title at 50
    Given a commit whose first message line is "x" repeated 90 times
    When I read its title
    And I read its short title
    Then the title is the letter "x" repeated 85 times then "... "
    And the short title is the letter "x" repeated 55 times then "... "
