Feature: Serving a blob raw (blob_plain headers)
  gitweb's git_blob_plain streams a blob's bytes verbatim under a Content-Type
  and a Content-Disposition it derives from the content and the file name. Text
  content (no NUL) is text/plain; binary content is image/png, image/gif or
  image/jpeg when the name carries that extension, and application/octet-stream
  otherwise — including when no name is given. Content wins over name: text is
  text whatever it is called. A charset is appended only to text/plain, and only
  when the site configures one. The download name is the file name, or the blob
  id (with a .txt suffix for text) when none is given. The disposition is inline
  unless XSS prevention is on and the type is neither a text type nor a safe
  image, in which case the blob is forced to an attachment download. The blob id
  used when no file name is given is "0a1b2c3".

  Scenario: text content is served as text/plain inline
    Given the bytes "68 69"
    When I serve it raw as "notes.txt"
    Then it is served as "text/plain"
    And it is offered inline as "notes.txt"

  Scenario: empty content is still text/plain
    Given the bytes ""
    When I serve it raw as "empty.txt"
    Then it is served as "text/plain"

  Scenario: one byte of text is still text/plain
    Given the bytes "78"
    When I serve it raw as "one.txt"
    Then it is served as "text/plain"

  Scenario: text content with no file name downloads as the blob id plus .txt
    Given the bytes "68 69"
    When I serve it raw with no file name
    Then it is served as "text/plain"
    And it is offered inline as "0a1b2c3.txt"

  Scenario: a PNG signature is served as image/png
    Given the bytes "89 50 00 47"
    When I serve it raw as "logo.png"
    Then it is served as "image/png"
    And it is offered inline as "logo.png"

  Scenario: an uppercase JPEG extension is served as image/jpeg
    Given the bytes "ff d8 00 e0"
    When I serve it raw as "PHOTO.JPEG"
    Then it is served as "image/jpeg"

  Scenario: a GIF extension is served as image/gif
    Given the bytes "47 49 00 46"
    When I serve it raw as "anim.gif"
    Then it is served as "image/gif"

  Scenario: binary content without an image extension is octet-stream
    Given the bytes "00 01 02"
    When I serve it raw as "a.out"
    Then it is served as "application/octet-stream"

  Scenario: binary content with no file name downloads as the bare blob id
    Given the bytes "00 01 02"
    When I serve it raw with no file name
    Then it is served as "application/octet-stream"
    And it is offered inline as "0a1b2c3"

  Scenario: a text file with an image extension stays text/plain
    Given the bytes "68 69"
    When I serve it raw as "notes.png"
    Then it is served as "text/plain"

  Scenario: a configured charset is appended to text/plain
    Given the bytes "68 69"
    When I serve it raw as "notes.txt" with charset "utf-8"
    Then it is served as "text/plain; charset=utf-8"

  Scenario: a configured charset is not appended to octet-stream
    Given the bytes "00 01 02"
    When I serve it raw as "a.out" with charset "utf-8"
    Then it is served as "application/octet-stream"

  Scenario: XSS prevention forces an attachment for octet-stream
    Given the bytes "00 01 02"
    When I serve it raw as "a.out" with XSS prevention
    Then it is offered as an attachment named "a.out"

  Scenario: XSS prevention still serves text/plain inline
    Given the bytes "68 69"
    When I serve it raw as "notes.txt" with XSS prevention
    Then it is offered inline as "notes.txt"

  Scenario: XSS prevention still serves a known image inline
    Given the bytes "89 50 00 47"
    When I serve it raw as "logo.png" with XSS prevention
    Then it is offered inline as "logo.png"
