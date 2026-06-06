Feature: Git identity lines (signatures)
  A commit or tag stores authorship as an ident line: a name, an optional
  email in angle brackets, a Unix epoch, and a timezone offset, e.g.
  "A U Thor <author@example.com> 1700000000 +0200".

  This mirrors gitweb's parse_commit_text: the trailing "<epoch> <tz>" is
  peeled off (the rightmost epoch wins, so digits in a name are safe), then
  the remaining ident is split into a name and the email between < and >.

  Scenario: A full ident parses into all four parts
    Given the ident line "A U Thor <author@example.com> 1700000000 +0200"
    When I parse the signature
    Then the author name is "A U Thor"
    And the email is "author@example.com"
    And the timestamp is 1700000000
    And the timezone is "+0200"

  Scenario: A western timezone offset is negative
    Given the ident line "A U Thor <author@example.com> 1700000000 -0700"
    When I parse the signature
    Then the timezone is "-0700"

  Scenario: Digits in the name do not get mistaken for the epoch
    Given the ident line "Agent 007 <bond@mi6.uk> 1234567890 +0000"
    When I parse the signature
    Then the author name is "Agent 007"
    And the email is "bond@mi6.uk"
    And the timestamp is 1234567890

  Scenario: An empty email is preserved, not dropped
    Given the ident line "Nobody <> 1700000000 +0000"
    When I parse the signature
    Then the author name is "Nobody"
    And the email is ""

  Scenario: An ident with no angle brackets has a name but no email
    Given the ident line "Anonymous Coward 1700000000 +0000"
    When I parse the signature
    Then the author name is "Anonymous Coward"
    And there is no email

  Scenario: A line with no timestamp is not a signature
    Given the ident line "Just A Name"
    When I parse the signature
    Then the signature is invalid
