Feature: Detecting binary content
  Blobs and diffs render differently for text and binary content: text inline,
  binary as a download. gitweb leans on Perl's -T heuristic; we use git's own
  rule instead — a NUL byte within the examined window marks binary content.
  This deliberately diverges from Perl's fuzzy high-bit count, which would
  misclassify perfectly good non-UTF-8 text as binary. Input is hex bytes.

  Scenario: Plain text has no NUL and is text
    Given the bytes "48 65 6c 6c 6f"
    When I check whether it is binary
    Then it is text

  Scenario: A NUL byte marks the content as binary
    Given the bytes "48 65 00 6c 6f"
    When I check whether it is binary
    Then it is binary

  Scenario: Empty content is treated as text
    Given the bytes ""
    When I check whether it is binary
    Then it is text

  Scenario: High-bit non-UTF-8 bytes without a NUL are still text
    Given the bytes "e9 e9 e9 e9"
    When I check whether it is binary
    Then it is text
