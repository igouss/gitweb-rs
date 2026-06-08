Feature: blob_plain serves a blob's raw bytes, byte-for-byte as gitweb
  gitweb's blob_plain action serves a blob's raw content — the bytes other tools
  download. It is a format-stable endpoint, so it is pinned by GOLDEN
  differential conformance: the body our gix adapter reads must equal the body
  the original gitweb.perl served, captured once into goldens/blob_plain.

  The body is opaque: it may be empty, a single byte, ordinary UTF-8 text,
  binary content with a NUL, or non-UTF-8 Latin-1. Every case must round-trip
  exactly through the capture, the commit, and our read — proving the harness is
  binary-safe, not just UTF-8-safe.

  The headers are differentially verified too: the media type our endpoint
  serves must equal the type gitweb chose (ignoring the charset CGI.pm bolts onto
  every type), and the Content-Disposition we emit must equal gitweb's verbatim.

  Background:
    Given the parity corpus

  Scenario Outline: a served blob is byte- and header-identical to gitweb's
    When I serve the "<blob>" blob plain
    Then its body matches gitweb's reference output
    And its media type matches gitweb's
    And its content disposition matches gitweb's

    Examples:
      | blob   |
      | empty  |
      | one    |
      | text   |
      | binary |
      | latin1 |

  Scenario: the captured reference is a well-formed CGI response
    When I serve the "text" blob plain
    Then the reference declares a Content-Type header
