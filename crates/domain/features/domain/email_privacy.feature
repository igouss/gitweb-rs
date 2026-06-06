Feature: Redacting email addresses
  When the email-privacy feature is on, gitweb hides addresses in commit and
  tag metadata: any "<local@domain>" is replaced with "<redacted>"
  (hide_mailaddrs_if_private). This rule is the redaction itself; whether it
  is applied is a configuration decision made by the caller.

  Scenario: An address in angle brackets is redacted
    Given the message line "A U Thor <author@example.com>"
    When I redact private emails
    Then the redacted line is "A U Thor <redacted>"

  Scenario: Redaction leaves the surrounding ident intact
    Given the message line "John Doe <john@doe.io> 1700000000 +0000"
    When I redact private emails
    Then the redacted line is "John Doe <redacted> 1700000000 +0000"

  Scenario: Every address on the line is redacted
    Given the message line "from <a@x.io> to <b@y.io>"
    When I redact private emails
    Then the redacted line is "from <redacted> to <redacted>"

  Scenario: A line with no address is unchanged
    Given the message line "no address here"
    When I redact private emails
    Then the redacted line is "no address here"

  Scenario: Bracketed text without an at-sign is not an address
    Given the message line "see <the-readme> for details"
    When I redact private emails
    Then the redacted line is "see <the-readme> for details"

  Scenario: An empty local part is not an address
    Given the message line "ping <@example.com>"
    When I redact private emails
    Then the redacted line is "ping <@example.com>"
