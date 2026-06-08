Feature: Serving one file (blob action)
  gitweb's git_blob opens the project, resolves a file within the base revision
  (HEAD by default), and serves it: a text file as numbered, anchorable lines; an
  image inline as <img>; any other binary as a raw download link only. A
  non-UTF-8 text file is decoded through the latin1 fallback; an empty file shows
  a note. A request that names no file is die_error(400); a file absent from the
  base, or a project that does not exist, is die_error(404).

  Scenario: a text file is served as numbered, anchorable lines
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=readme.txt"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "hello world"
    And the response body contains "second line"
    And the response body contains "#l1"

  Scenario: a non-UTF8 file is decoded through the latin1 fallback
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=latin1.txt"
    Then the response status is 200
    And the response body contains "Héllo"

  Scenario: an empty file shows a note
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=empty.txt"
    Then the response status is 200
    And the response body contains "Empty file."

  Scenario: a binary file is offered as a raw download, not inlined
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=data.bin"
    Then the response status is 200
    And the response body contains "Download raw"
    And the response body contains "a=blob_plain"

  Scenario: an image file is shown inline
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=logo.png"
    Then the response status is 200
    And the response body contains "<img"
    And the response body contains "logo.png"

  Scenario: a request with no file name is a bad request
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob"
    Then the response status is 400

  Scenario: a file absent from the base is not found
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=ghost"
    Then the response status is 404

  Scenario: the blob for a project that does not exist is not found
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=ghost.git&a=blob&hb=HEAD&f=readme.txt"
    Then the response status is 404
