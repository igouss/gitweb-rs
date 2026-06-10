Feature: git binary delta encoding (diff-delta.c)
  git's `diff_delta(source, target)` produces the binary delta that packfiles and
  `git diff --binary` use: a LEB128 header (source size, then target size) followed
  by COPY ops (top bit set — copy a run from the source) and INSERT ops (top bit
  clear — the opcode IS the count of inline literal bytes). The encoder is a
  Rabin-fingerprint-indexed greedy matcher; the exact bytes it emits are the spec
  for a byte-exact GIT binary patch body, so these scenarios pin git's real output.

  Every expected delta below was captured from real git via
  `test-tool delta -d <source> <target> <out>` (see the bead's capture script);
  the deltas are byte-for-byte what git's create_delta() emits.

  # --- zero copies: pure insert (no source window matches) ---

  Scenario: a target shorter than the window is one insert op
    Given a delta source of 16 bytes valued 0x61
    And a delta target of text "bbbb"
    When I compute the binary delta
    Then the delta is bytes "10 04 04 62 62 62 62"

  Scenario: an insert run flushes at the 0x7f opcode cap and continues
    Given a delta source of 16 bytes valued 0x61
    And a delta target of 200 ramp bytes
    When I compute the binary delta
    Then the delta is bytes "10 c8 01 7f 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12 13 14 15 16 17 18 19 1a 1b 1c 1d 1e 1f 20 21 22 23 24 25 26 27 28 29 2a 2b 2c 2d 2e 2f 30 31 32 33 34 35 36 37 38 39 3a 3b 3c 3d 3e 3f 40 41 42 43 44 45 46 47 48 49 4a 4b 4c 4d 4e 4f 50 51 52 53 54 55 56 57 58 59 5a 5b 5c 5d 5e 5f 60 61 62 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f 70 71 72 73 74 75 76 77 78 79 7a 7b 7c 7d 7e 49 7f 80 81 82 83 84 85 86 87 88 89 8a 8b 8c 8d 8e 8f 90 91 92 93 94 95 96 97 98 99 9a 9b 9c 9d 9e 9f a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 aa ab ac ad ae af b0 b1 b2 b3 b4 b5 b6 b7 b8 b9 ba bb bc bd be bf c0 c1 c2 c3 c4 c5 c6 c7"

  # --- one copy: the matcher finds a run and back-extends over the primed bytes ---

  Scenario: an identical target is a single whole-buffer copy (prime back-extended)
    Given a delta source of text "The quick brown fox jumps over!!"
    And the delta target equals the delta source
    When I compute the binary delta
    Then the delta is bytes "20 20 90 20"

  Scenario: a shared prefix copies, the changed tail inserts
    Given a delta source of text "The quick brown fox jumps over the lazy dog."
    And a delta target of text "The quick brown fox JUMPED over the lazy dog."
    When I compute the binary delta
    Then the delta is bytes "2c 2d 90 14 19 4a 55 4d 50 45 44 20 6f 76 65 72 20 74 68 65 20 6c 61 7a 79 20 64 6f 67 2e"

  Scenario: a copy at a non-zero offset emits an offset byte and back-extends
    Given a delta source of text "PREFIX-the common suffix block long enough to match"
    And a delta target of text "X-the common suffix block long enough to match"
    When I compute the binary delta
    Then the delta is bytes "33 2e 91 05 2e"

  # --- partial back-extension: some leading literals stay, the rest are absorbed ---

  Scenario: back-extension stops at the source start, leaving a short insert
    Given a delta source of text "pqthe_quick_brown_fox_jumped_over"
    And a delta target of text "ABCpqthe_quick_brown_fox_jumped_over"
    When I compute the binary delta
    Then the delta is bytes "21 24 03 41 42 43 90 21"

  # --- two distinct copies: a copy, an insert, then a second copy (the common diff shape) ---

  Scenario: two separated common runs copy independently around an inserted gap
    Given a delta source of text "abcdefghijklmnopqrstuvwxyz0123456789ABCDzyxwvutsrqponmlkjihgfedcba9876543210WXYZ"
    And a delta target of text "abcdefghijklmnopqrstuvwxyz0123456789ABCDMIDzyxwvutsrqponmlkjihgfedcba9876543210WXYZ"
    When I compute the binary delta
    Then the delta is bytes "50 53 90 28 02 4d 49 91 27 29"

  # --- many copies: a match over 64KB splits into capped copy ops ---

  Scenario: a 65541-byte identical buffer splits into a 0x10000 copy plus the rest
    Given a delta source of 65541 ramp bytes
    And the delta target equals the delta source
    When I compute the binary delta
    Then the delta is bytes "85 80 04 85 80 04 80 94 01 05"

  # --- a copy whose offset and size each need two bytes to encode ---

  Scenario: a copy from a high offset of a large size packs two offset and two size bytes
    Given a delta source of 320 bytes valued 0xff
    And 400 ramp bytes are appended to the delta source
    And a delta target of 336 ramp bytes
    When I compute the binary delta
    Then the delta is bytes "d0 05 d0 02 b3 40 01 50 01"

  # --- empty buffers: git cannot index/delta them, so no delta (literal fallback) ---

  Scenario: an empty source yields no delta (file creation)
    Given a delta source of text ""
    And a delta target of text "abc"
    When I compute the binary delta
    Then there is no delta

  Scenario: an empty target yields no delta (file deletion)
    Given a delta source of text "abc"
    And a delta target of text ""
    When I compute the binary delta
    Then there is no delta

  # --- size-bounded delta: git's diff_delta(max_size), the GIT binary patch cap ---
  # The binary patch body computes a delta capped at the deflated literal's size,
  # and keeps it only if it (deflated) still wins — so the encoder must abandon a
  # delta that outgrows the cap exactly as git does.

  Scenario: a delta within the size cap is returned
    Given a delta source of text "The quick brown fox jumps over!!"
    And the delta target equals the delta source
    When I compute the binary delta capped at 4 bytes
    Then the delta is bytes "20 20 90 20"

  Scenario: a delta that outgrows the size cap is abandoned
    Given a delta source of text "The quick brown fox jumps over!!"
    And the delta target equals the delta source
    When I compute the binary delta capped at 3 bytes
    Then there is no delta

  Scenario: a zero cap means unbounded
    Given a delta source of text "The quick brown fox jumps over!!"
    And the delta target equals the delta source
    When I compute the binary delta capped at 0 bytes
    Then the delta is bytes "20 20 90 20"
