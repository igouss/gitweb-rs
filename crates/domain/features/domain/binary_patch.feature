Feature: git GIT binary patch body (emit_binary_diff)
  git's `format-patch --binary` embeds a binary file as a `GIT binary patch` so the
  mail is `git am`-able. For one file git writes the header then two blocks: a
  forward block rebuilding the post-image from the pre-image, then a reverse block
  rebuilding the pre-image from the post-image. Each block emits whichever is
  shorter of a deflated literal (`literal <post-size>`) or a deflated binary delta
  (`delta <uncompressed-delta-size>`), base85-framed and closed by a blank line.

  Every expected body below was captured from real `git format-patch --binary`
  (git 2.54.0): the deflate is plain zlib level 1, the delta is git's diff-delta.c
  encoder, and the literal-vs-delta choice is git's own size heuristic.

  Scenario: a small dissimilar modification keeps literals on both sides
    Given a binary pre-image of bytes "00 01 02 03 04"
    And a binary post-image of bytes "00 01 02 03 04 05 06"
    When I render the binary patch body
    Then the binary patch body is:
      """
      GIT binary patch
      literal 7
      OcmZQzWMXDvWdi^JKL8d0

      literal 5
      McmZQzWMXCk000>P3jhEB

      """

  Scenario: a binary creation has no pre-image, so both blocks are literal
    Given a binary pre-image of bytes ""
    And a binary post-image of bytes "00 01 02 03"
    When I render the binary patch body
    Then the binary patch body is:
      """
      GIT binary patch
      literal 4
      LcmZQzWMT#Y01f~L

      literal 0
      HcmV?d00001

      """

  Scenario: a large mostly-similar modification lets the delta win on both sides
    Given a binary pre-image of the delta-wins source
    And a binary post-image of the delta-wins target
    When I render the binary patch body
    Then the binary patch body is:
      """
      GIT binary patch
      delta 13
      Ucmcb?a)V`q4kM$*MqO=203iGW*#H0l

      delta 13
      Ucmcb?a)V`q4&y={#)&#Q03#U$O#lD@

      """
