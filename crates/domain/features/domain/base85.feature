Feature: git base85 encoding for the GIT binary patch body
  git's `format-patch --binary` encodes the deflated literal (or delta) of a
  binary blob in git's base85 variant — an 85-character alphabet packed big-endian
  four bytes to five digits (a short final group zero-padded but still five
  digits), wrapped in the patch body's line framing: at most 52 data bytes per
  line, each line prefixed by a length marker (1..26 → A..Z, 27..52 → a..z) and
  newline-terminated. These scenarios pin git's `encode_85` (base85.c) plus
  `emit_binary_diff_body`'s line loop (diff.c); every expected string was captured
  from real git output.

  Scenario: one byte fills a single five-digit group, zero-padded
    Given base85 input bytes "ff"
    When I base85-encode them
    Then the base85 framing is "A{{R30"

  Scenario: four bytes are exactly one five-digit group
    Given base85 input bytes "de ad be ef"
    When I base85-encode them
    Then the base85 framing is "D-mSjx"

  Scenario: five bytes span two groups, the second zero-padded
    Given base85 input bytes "01 02 03 04 05"
    When I base85-encode them
    Then the base85 framing is "E0RjUA1poj5"

  Scenario: a 15-byte deflated literal is one line marked by its byte count
    Given base85 input bytes "78 01 63 60 64 62 66 61 65 03 00 00 3f 00 16"
    When I base85-encode them
    Then the base85 framing is "OcmZQzWMXDvWdi^JKL8d0"

  Scenario: a full 52-byte line uses the lowercase marker z
    Given base85 input bytes "01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12 13 14 15 16 17 18 19 1a 1b 1c 1d 1e 1f 20 21 22 23 24 25 26 27 28 29 2a 2b 2c 2d 2e 2f 30 31 32 33 34"
    When I base85-encode them
    Then the base85 framing is "z0RjUA1qKHQ2?`4g4Gs?w5fT#=6&4p585$cL9UdPbAtECrB_<~*DJm;0EiNxGF)}kW"

  Scenario: a 53rd byte starts a second line
    Given base85 input bytes "01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12 13 14 15 16 17 18 19 1a 1b 1c 1d 1e 1f 20 21 22 23 24 25 26 27 28 29 2a 2b 2c 2d 2e 2f 30 31 32 33 34 35"
    When I base85-encode them
    Then the base85 framing has lines:
      """
      z0RjUA1qKHQ2?`4g4Gs?w5fT#=6&4p585$cL9UdPbAtECrB_<~*DJm;0EiNxGF)}kW
      AH2?qr
      """
