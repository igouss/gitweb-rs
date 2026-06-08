Feature: Serving one file raw (blob_plain)
  gitweb's git_blob_plain finds a file exactly as git_blob does — by its id, or
  by resolving a path under the base revision (HEAD by default) — then serves its
  bytes verbatim under a Content-Type and a Content-Disposition derived from the
  content and the file name. The bytes are never transformed: a non-UTF-8 file
  comes back untouched. A request that names neither an id nor a path is a bad
  request; a path absent from the base is not found.

  Background:
    Given the blob base is commit "Initial import"

  Scenario: a text file is served as text/plain, inline, bytes verbatim
    Given the blob "readme" at "README" contains text "hello\nworld"
    When I serve the blob raw at path "README"
    Then the raw blob is served as "text/plain"
    And the raw blob is offered inline as "README"
    And the raw blob body is "hello\nworld"

  Scenario: a binary file is served as application/octet-stream
    Given the blob "data" at "data.bin" contains bytes "00 01 02"
    When I serve the blob raw at path "data.bin"
    Then the raw blob is served as "application/octet-stream"
    And the raw blob is offered inline as "data.bin"

  Scenario: an image file is served as its image type
    Given the blob "logo" at "logo.png" contains bytes "89 50 00 47"
    When I serve the blob raw at path "logo.png"
    Then the raw blob is served as "image/png"

  Scenario: a non-UTF8 file's bytes pass through untouched
    Given the blob "latin" at "latin1.txt" contains bytes "63 61 66 e9"
    When I serve the blob raw at path "latin1.txt"
    Then the raw blob is served as "text/plain"
    And the raw blob body has bytes "63 61 66 e9"

  Scenario: a file served by id alone is still served as text/plain
    Given the blob "readme" at "README" contains text "loose"
    When I serve the blob raw of id "readme"
    Then the raw blob is served as "text/plain"

  Scenario: a request with neither id nor path is a bad request
    When I serve the blob raw with neither id nor path
    Then serving the raw blob fails as invalid

  Scenario: a path absent from the base is not found
    When I serve the blob raw at path "ghost"
    Then serving the raw blob fails as not found
