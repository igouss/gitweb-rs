Feature: Search pattern matching
  gitweb's commit/author/committer search matches a pattern against text the way
  git does: by default the pattern is a fixed string matched without regard to
  case (git's `--fixed-strings --regexp-ignore-case`), but when the user checks
  the "re" box (search_use_regexp) the pattern is a POSIX extended regular
  expression, still case-insensitive (`--extended-regexp`). An ill-formed regular
  expression is rejected the way gitweb dies 400 "Invalid search regexp". The
  matcher also reports where the first match begins and ends, so the result rows
  can highlight the matching fragment of each line.

  Scenario: a fixed pattern matches a substring regardless of case
    Given the fixed search pattern "banana"
    Then it matches "Banana bread is great"
    And it does not match "apple pie"

  Scenario: a fixed pattern treats regex metacharacters literally
    Given the fixed search pattern "a.c"
    Then it matches "the a.c file"
    And it does not match "abc def"

  Scenario: a regexp pattern matches by regular expression, case-insensitively
    Given the regexp search pattern "ba.a.a"
    Then it matches "BANANA split"
    And it does not match "bandana"

  Scenario: an invalid regexp pattern is rejected
    Given the regexp search pattern "a(b" is rejected as invalid

  Scenario: the matcher reports the first match span
    Given the fixed search pattern "cat"
    Then the first match in "a cat and a cat" spans bytes 2 to 5
