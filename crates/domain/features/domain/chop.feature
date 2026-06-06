Feature: Word-aware string truncation
  Long titles and paths are shortened for display, mirroring gitweb's
  chop_str. Truncation is character-based (never splitting a UTF-8 byte
  sequence) and word-aware: a word that would fit within "slack" extra
  characters is kept whole rather than cut.

  When a string is shortened, an ellipsis filler marks where: a trailing
  "... " on the right, a leading " ..." on the left, and " ... " in the
  middle for center truncation.

  Scenario: A string no longer than the limit plus filler is left alone
    Given the text "Hello, World"
    When I right-chop it to 10 keeping 10 extra
    Then the chopped text is "Hello, World"

  Scenario: Right truncation keeps a straddling word whole and appends a filler
    Given the text "The quick brown fox jumps over the lazy dog"
    When I right-chop it to 10 keeping 10 extra
    Then the chopped text is "The quick brown... "

  Scenario: A word longer than the slack is cut mid-word
    Given the text "supercalifragilistic expialidocious"
    When I right-chop it to 5 keeping 3 extra
    Then the chopped text is "supercal... "

  Scenario: Left truncation prepends a filler before the kept tail
    Given the text "abcdefghij klmnopqrst"
    When I left-chop it to 5 keeping 10 extra
    Then the chopped text is " ...klmnopqrst"

  Scenario: Center truncation keeps both ends and elides the middle
    Given the text "abcdefghijklmnopqrstuvwxyz0123456789"
    When I center-chop it to 10 keeping 5 extra
    Then the chopped text is "abcdefghij ... 0123456789"

  Scenario: Truncation counts characters, not bytes
    Given the text "ααααα βββββ γγγγγ δδδδδ"
    When I right-chop it to 5 keeping 10 extra
    Then the chopped text is "ααααα... "
