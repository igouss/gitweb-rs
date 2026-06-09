Feature: The content-grep matcher (search_use_regexp)
  gitweb's `git_search_files` hands git-grep either `-F` (the default: the pattern
  is a literal string, matched case-SENSITIVELY) or `-E -i` (when the user checks
  the *re* box: a POSIX extended regular expression, matched case-INSENSITIVELY).
  That is the one rule this value object captures, over raw bytes so it serves
  both a text line and a whole binary file the same way. It is deliberately a
  DIFFERENT matcher from the commit-search `SearchPattern`, which is always
  case-insensitive — git-grep's fixed mode is case-sensitive, as gitweb's own help
  text warns.

  In these scenarios the escape "\0" is a NUL byte, so a binary haystack's exact
  bytes are pinned.

  # --- fixed mode: literal, case-sensitive (git grep -F) ---

  Scenario: a fixed pattern matches a literal byte substring
    Given a fixed grep pattern "lpha"
    Then the grep pattern matches "alphabet"

  Scenario: a fixed pattern is case-sensitive
    Given a fixed grep pattern "foo"
    Then the grep pattern does not match "FOO"

  Scenario: a fixed pattern treats metacharacters literally
    Given a fixed grep pattern "a.c"
    Then the grep pattern does not match "abc"

  Scenario: a fixed pattern matches its literal metacharacters
    Given a fixed grep pattern "a.c"
    Then the grep pattern matches "xa.cy"

  Scenario: a fixed pattern that occurs nowhere does not match
    Given a fixed grep pattern "zzz"
    Then the grep pattern does not match "alphabet"

  Scenario: an empty fixed pattern matches nothing
    Given a fixed grep pattern ""
    Then the grep pattern does not match "anything"

  Scenario: a fixed pattern matches inside binary bytes
    Given a fixed grep pattern "secret"
    Then the grep pattern matches "head\0secret\0tail"

  # --- regexp mode: POSIX ERE, case-insensitive (git grep -E -i) ---

  Scenario: a regexp pattern matches case-insensitively
    Given a regexp grep pattern "foo"
    Then the grep pattern matches "FOO"

  Scenario: a regexp metacharacter matches as a regular expression
    Given a regexp grep pattern "a.c"
    Then the grep pattern matches "abc"

  Scenario: a regexp alternation matches either branch
    Given a regexp grep pattern "foo|bar"
    Then the grep pattern matches "a bar here"

  Scenario: a regexp anchor binds to the bytes given
    Given a regexp grep pattern "^foo"
    Then the grep pattern does not match "a foo"

  Scenario: a regexp that matches nothing in the bytes does not match
    Given a regexp grep pattern "z+q"
    Then the grep pattern does not match "alphabet"

  # --- a malformed regexp is rejected (gitweb's 400 "Invalid search regexp") ---

  Scenario: an unbalanced regexp is rejected
    Given the grep pattern build for regexp "a(" is attempted
    Then building the grep pattern fails

  Scenario: a well-formed regexp builds
    Given the grep pattern build for regexp "a(b|c)" is attempted
    Then building the grep pattern succeeds
