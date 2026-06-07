Feature: URL-decoding request and file tokens
  gitweb reads the projects-list file and incoming request parameters with
  CGI::Util::unescape: percent-escapes become their byte, and a plus becomes a
  space. The projects-list file leans on this — `git%2Fgit.git Linus+Torvalds`
  is the project `git/git.git` owned by `Linus Torvalds`.

  Decoded percent-bytes are reassembled as UTF-8; an escape that is not two hex
  digits is left exactly as written, matching CGI::Util's lenient behaviour.

  Scenario: text with nothing to decode is returned unchanged
    Given the encoded token "git.git"
    When I unescape it
    Then the decoded token is "git.git"

  Scenario: a plus becomes a space
    Given the encoded token "Linus+Torvalds"
    When I unescape it
    Then the decoded token is "Linus Torvalds"

  Scenario: a single percent-escape decodes to its character
    Given the encoded token "a%2Fb"
    When I unescape it
    Then the decoded token is "a/b"

  Scenario: several escapes and pluses decode together
    Given the encoded token "H.+Peter+Anvin%21"
    When I unescape it
    Then the decoded token is "H. Peter Anvin!"

  Scenario: multi-byte UTF-8 is reassembled from its percent-bytes
    Given the encoded token "%C3%A9"
    When I unescape it
    Then the decoded token is "é"

  Scenario: an incomplete escape is left exactly as written
    Given the encoded token "100%"
    When I unescape it
    Then the decoded token is "100%"

  Scenario: a non-hex escape is left exactly as written
    Given the encoded token "%zz"
    When I unescape it
    Then the decoded token is "%zz"
