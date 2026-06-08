Feature: Log comment body (git_print_log)
  gitweb's git_log_body renders a commit's message through git_print_log: it drops
  the message's leading blank lines, collapses any run of blank lines to a single
  one, classifies sign-off / trailer lines so they can be styled apart (and drops
  the blank line that follows one), recognises an "xxxlink: <url>" trailer as a
  link, and ends the block with one trailing blank line unless the last thing was
  already blank or a trailer.

  This is a pure rule over the message text. The SHA-token linkification
  git_print_log also does (format_log_line_html, which turns abbreviated object
  ids into links to the object view) needs the object action's URL and is tracked
  separately, like the ref markers and avatars carved off the shortlog.

  Scenario: an empty message is a single trailing blank line
    Given an empty commit message
    When I split the log body
    Then the log body has 1 line
    And log line 1 is blank

  Scenario: a one-line message keeps the line then a trailing blank
    Given the commit message "Fix the thing"
    When I split the log body
    Then the log body has 2 lines
    And log line 1 is text "Fix the thing"
    And log line 2 is blank

  Scenario: leading blank lines are dropped
    Given the commit message:
      """

      Real subject
      """
    When I split the log body
    Then the log body has 2 lines
    And log line 1 is text "Real subject"
    And log line 2 is blank

  Scenario: a run of blank lines collapses to one
    Given the commit message:
      """
      Subject

      Body
      """
    When I split the log body
    Then the log body has 4 lines
    And log line 1 is text "Subject"
    And log line 2 is blank
    And log line 3 is text "Body"
    And log line 4 is blank

  Scenario: a sign-off line is classified apart
    Given the commit message:
      """
      Subject

      Signed-off-by: Ada Lovelace <ada@example.com>
      """
    When I split the log body
    Then the log body has 3 lines
    And log line 1 is text "Subject"
    And log line 2 is blank
    And log line 3 is a sign-off "Signed-off-by: Ada Lovelace <ada@example.com>"

  Scenario: a sign-off suppresses the blank line that follows it
    Given the commit message:
      """
      Subject

      Acked-by: Bob

      """
    When I split the log body
    Then the log body has 3 lines
    And log line 3 is a sign-off "Acked-by: Bob"

  Scenario: Closes and Fixes and Cc count as trailers
    Given the commit message:
      """
      Subject
      Cc: maintainer
      Closes: 123
      Fixes: 456
      """
    When I split the log body
    Then the log body has 4 lines
    And log line 2 is a sign-off "Cc: maintainer"
    And log line 3 is a sign-off "Closes: 123"
    And log line 4 is a sign-off "Fixes: 456"

  Scenario: a colon line that is not a known trailer stays plain text
    Given the commit message:
      """
      Subject
      Note: this is just prose
      """
    When I split the log body
    Then the log body has 3 lines
    And log line 2 is text "Note: this is just prose"

  Scenario: an xxxlink trailer links its URL
    Given the commit message:
      """
      Subject

      Link: https://example.com/issue/1
      """
    When I split the log body
    Then the log body has 3 lines
    And log line 3 is an autolink labelled "Link" to "https://example.com/issue/1"
