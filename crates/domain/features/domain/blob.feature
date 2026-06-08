Feature: Classifying a blob for display
  gitweb's git_blob shows a blob three ways. Text content is rendered inline
  line by line; an image is shown inline as an <img>; any other binary content
  is not inlined at all and is offered as a raw download. The text-vs-binary
  split is the NUL rule (model/binary): content with a NUL is binary. Among
  binary blobs, an image is recognised by the file name's extension
  (png/gif/jpeg) — gitweb's built-in mimetype fallback. A blob whose content is
  text is always text, whatever its name. Input is space-separated hex bytes.

  Scenario: text content is shown inline
    Given the bytes "48 69"
    When I classify the blob as "notes.txt"
    Then the blob displays as text

  Scenario: text content with no file name is still text
    Given the bytes "48 69"
    When I classify the blob with no file name
    Then the blob displays as text

  Scenario: a text file with an image extension is still text
    Given the bytes "48 69"
    When I classify the blob as "notes.png"
    Then the blob displays as text

  Scenario: binary content with an image extension is an image
    Given the bytes "89 50 00 47"
    When I classify the blob as "logo.png"
    Then the blob displays as an image

  Scenario: an uppercase jpeg extension still counts as an image
    Given the bytes "ff d8 00 e0"
    When I classify the blob as "PHOTO.JPEG"
    Then the blob displays as an image

  Scenario: binary content without an image extension is a download
    Given the bytes "00 01 02"
    When I classify the blob as "a.out"
    Then the blob displays as binary

  Scenario: binary content with no file name is a download
    Given the bytes "00 01 02"
    When I classify the blob with no file name
    Then the blob displays as binary
