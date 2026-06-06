Feature: Decoding git bytes to text
  Git stores blob and metadata bytes with no guaranteed encoding. gitweb's
  to_utf8 keeps the bytes if they are already valid UTF-8, otherwise decodes
  them through a fallback encoding (latin1 by default). The input is given
  here as space-separated hex bytes.

  Scenario: Valid UTF-8 ASCII passes through unchanged
    Given the bytes "48 65 6c 6c 6f"
    When I decode them with the latin1 fallback
    Then the text is "Hello"

  Scenario: Valid multi-byte UTF-8 is preserved
    Given the bytes "68 c3 a9 6c 6c 6f"
    When I decode them with the latin1 fallback
    Then the text is "héllo"

  Scenario: Invalid UTF-8 is decoded through the latin1 fallback
    Given the bytes "48 e9 6c 6c 6f"
    When I decode them with the latin1 fallback
    Then the text is "Héllo"

  Scenario: Empty input decodes to the empty string
    Given the bytes ""
    When I decode them with the latin1 fallback
    Then the text is ""
