Feature: Serving one file raw (blob_plain action)
  gitweb's git_blob_plain streams a file's bytes verbatim. It opens the project,
  resolves the file within the base revision (HEAD by default), and serves the
  bytes under a Content-Type and a Content-Disposition it derives from the
  content and the name: text/plain for text, application/octet-stream for binary,
  image/png for an image — each offered inline under the file's name. A request
  naming no file is die_error(400); a file absent from the base, or a project
  that does not exist, is die_error(404).

  Scenario: a text file is served as text/plain inline with its bytes verbatim
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain&hb=HEAD&f=readme.txt"
    Then the response status is 200
    And the response content type is "text/plain"
    And the response is offered inline as "readme.txt"
    And the response body contains "hello world"
    And the response body contains "second line"

  Scenario: a binary file is served as application/octet-stream inline
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain&hb=HEAD&f=data.bin"
    Then the response status is 200
    And the response content type is "application/octet-stream"
    And the response is offered inline as "data.bin"

  Scenario: an image file is served as its image type
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain&hb=HEAD&f=logo.png"
    Then the response status is 200
    And the response content type is "image/png"
    And the response is offered inline as "logo.png"

  Scenario: a non-UTF8 file is served as text/plain, bytes untouched
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain&hb=HEAD&f=latin1.txt"
    Then the response status is 200
    And the response content type is "text/plain"
    And the response is offered inline as "latin1.txt"

  Scenario: a request with no file name is a bad request
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain"
    Then the response status is 400

  Scenario: a file absent from the base is not found
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=t.git&a=blob_plain&hb=HEAD&f=ghost"
    Then the response status is 404

  Scenario: the raw blob for a project that does not exist is not found
    Given a repository "t.git" with blobs
    And the blob_plain action is served
    When I GET "/?p=ghost.git&a=blob_plain&hb=HEAD&f=readme.txt"
    Then the response status is 404
