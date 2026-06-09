Feature: Content-grep matches within one file
  gitweb's `git_search_files` greps each tracked file at one revision with
  `git grep -n -z <pattern>` in one of two modes: the default `-F` (a literal,
  case-sensitive substring) or `-E -i` when the *re* box is checked (a
  case-insensitive POSIX extended regular expression). This is the pure per-file
  rule behind it — given a file's bytes and a pattern, what matches does git
  list? A text file lists one hit per line that matches, with the line's 1-based
  number and its raw text; a binary file is listed once as
  "Binary file <path> matches" with no line text; whichever mode is in play, a
  line is listed at most once however often the pattern occurs in it. The tree
  walk, the blob reads, the cross-file order, and gitweb's 1000-match cap are the
  adapter's job, verified by conformance.

  In these scenarios the escape "\n" is a newline and "\0" is a NUL byte, so a
  file's exact bytes — including whether it ends in a newline — are pinned.

  # --- no matches ---

  Scenario: a pattern that occurs in no line is not found
    Given a file "notes.txt" with content "alpha\nbeta\n"
    When I grep "gamma"
    Then 0 grep matches are found

  Scenario: an empty file matches nothing
    Given a file "empty.txt" with content ""
    When I grep "anything"
    Then 0 grep matches are found

  Scenario: an empty pattern matches nothing
    Given a file "notes.txt" with content "alpha\nbeta\n"
    When I grep ""
    Then 0 grep matches are found

  # --- one match ---

  Scenario: a pattern in one line is listed with its 1-based number and text
    Given a file "notes.txt" with content "alpha\nbeta\ngamma\n"
    When I grep "beta"
    Then 1 grep match is found
    And grep match 0 is line 2 "beta" in "notes.txt"

  Scenario: the first line is numbered 1
    Given a file "notes.txt" with content "alpha\nbeta\n"
    When I grep "alpha"
    Then 1 grep match is found
    And grep match 0 is line 1 "alpha" in "notes.txt"

  Scenario: a final line without a trailing newline still matches and counts
    Given a file "notes.txt" with content "alpha\nbeta"
    When I grep "beta"
    Then 1 grep match is found
    And grep match 0 is line 2 "beta" in "notes.txt"

  # --- many matches ---

  Scenario: each matching line is listed in order
    Given a file "notes.txt" with content "foo\nbar\nfoo baz\n"
    When I grep "foo"
    Then 2 grep matches are found
    And grep match 0 is line 1 "foo" in "notes.txt"
    And grep match 1 is line 3 "foo baz" in "notes.txt"

  Scenario: a line with the pattern twice is listed once, not per occurrence
    Given a file "notes.txt" with content "foofoo\nbar\n"
    When I grep "foo"
    Then 1 grep match is found
    And grep match 0 is line 1 "foofoo" in "notes.txt"

  # --- case sensitivity: -F without -i ---

  Scenario: a lowercase pattern does not match an uppercase line
    Given a file "notes.txt" with content "Foo\nfoo\n"
    When I grep "foo"
    Then 1 grep match is found
    And grep match 0 is line 2 "foo" in "notes.txt"

  Scenario: the same pattern uppercased matches the other line
    Given a file "notes.txt" with content "Foo\nfoo\n"
    When I grep "Foo"
    Then 1 grep match is found
    And grep match 0 is line 1 "Foo" in "notes.txt"

  # --- binary files: reported once, no line text ---

  Scenario: a binary file whose bytes contain the pattern is reported once
    Given a file "data.bin" with content "head\0secret\0tail"
    When I grep "secret"
    Then 1 grep match is found
    And grep match 0 is binary file "data.bin"

  Scenario: a binary file whose bytes lack the pattern is not reported
    Given a file "data.bin" with content "head\0tail"
    When I grep "secret"
    Then 0 grep matches are found

  # --- non-UTF-8 content decoded through the fallback encoding ---

  Scenario: a non-UTF-8 line matches and its text is decoded via latin1
    Given a file "notes.txt" with latin1 content "café au lait\n"
    When I grep "caf"
    Then 1 grep match is found
    And grep match 0 is line 1 "café au lait" in "notes.txt"

  # --- regexp mode: -E -i (case-insensitive POSIX ERE) ---

  Scenario: a regexp pattern matches a line case-insensitively
    Given a file "notes.txt" with content "Foo\nbar\n"
    When I grep regexp "foo"
    Then 1 grep match is found
    And grep match 0 is line 1 "Foo" in "notes.txt"

  Scenario: a regexp metacharacter matches as a regular expression, not a literal
    Given a file "notes.txt" with content "abc\na.c\nxyz\n"
    When I grep regexp "a.c"
    Then 2 grep matches are found
    And grep match 0 is line 1 "abc" in "notes.txt"
    And grep match 1 is line 2 "a.c" in "notes.txt"

  Scenario: a regexp anchor binds to each line
    Given a file "notes.txt" with content "foo\nafoo\n"
    When I grep regexp "^foo"
    Then 1 grep match is found
    And grep match 0 is line 1 "foo" in "notes.txt"

  Scenario: a regexp matches inside binary bytes case-insensitively
    Given a file "data.bin" with content "head\0SECRET\0tail"
    When I grep regexp "secret"
    Then 1 grep match is found
    And grep match 0 is binary file "data.bin"
