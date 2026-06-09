Feature: The pickaxe matcher (PickaxePattern)
  gitweb's pickaxe search (git_search_changes) feeds git `log -S<text>`, plus
  `--pickaxe-regex` when search_use_regexp is set. Unlike the message / author /
  committer search — which is always case-insensitive (`-i`) — pickaxe is
  **case-sensitive** in both modes, and it does not test mere presence: it counts
  the *number of occurrences* of the pattern in a file's bytes, listing a commit
  only when that count changes against the parent. This value object captures
  exactly that one rule, over raw bytes, in either mode:

    - fixed (`-S`):          a literal byte substring, counted non-overlapping
    - regexp (`--pickaxe-regex`): a POSIX ERE, its matches counted non-overlapping

  Both are case-sensitive; a malformed regexp is rejected (gitweb's 400). The
  escape "\t" in a haystack below is a tab.

  # --- fixed mode: non-overlapping, case-sensitive occurrence count ---

  Scenario: a fixed pattern absent from the bytes counts zero
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern counts 0 in "apple pie"

  Scenario: a fixed pattern present once counts one
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern counts 1 in "banana bread"

  Scenario: a fixed pattern present several times counts each
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern counts 3 in "banana banana banana"

  Scenario: overlapping fixed occurrences are counted non-overlapping, as git does
    Given a fixed pickaxe pattern "aa"
    Then the pickaxe pattern counts 2 in "aaaa"

  Scenario: a fixed pattern is matched case-sensitively
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern counts 0 in "BANANA SPLIT"

  # --- the change test: a commit is a hit only when the count differs ---

  Scenario: adding the pattern is a change
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern reports a change from "plain bread" to "banana bread"

  Scenario: removing the pattern is a change
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern reports a change from "banana bread" to "plain bread"

  Scenario: editing a file without changing the count is not a change
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern reports no change from "banana bread" to "banana cake"

  Scenario: an untouched file is not a change
    Given a fixed pickaxe pattern "banana"
    Then the pickaxe pattern reports no change from "no fruit here" to "no fruit here"

  # --- regexp mode (--pickaxe-regex): still case-sensitive, matches counted ---

  Scenario: a regexp pattern counts its non-overlapping matches
    Given a regexp pickaxe pattern "a[0-9]"
    Then the pickaxe pattern counts 3 in "a1 then a2 then a9"

  Scenario: a regexp pattern is matched case-sensitively
    Given a regexp pickaxe pattern "[A-Z]+"
    Then the pickaxe pattern counts 1 in "lower UPPER lower"

  Scenario: a regexp change is detected by its match count
    Given a regexp pickaxe pattern "TODO|FIXME"
    Then the pickaxe pattern reports a change from "clean code" to "TODO fix this FIXME later"

  # --- regexp validation ---

  Scenario: a malformed regexp is rejected
    Given the pickaxe pattern build for regexp "a(b" is attempted
    Then building the pickaxe pattern fails

  Scenario: a well-formed regexp builds
    Given the pickaxe pattern build for regexp "a.c" is attempted
    Then building the pickaxe pattern succeeds
