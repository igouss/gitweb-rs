Feature: Showing one file (blob)
  gitweb's git_blob shows one file of a revision. The blob is found by its id, or
  by resolving a path within the base revision (HEAD by default). Text content is
  decoded to UTF-8 with a latin1 fallback and split into lines; an image is not
  decoded but marked for inline display; any other binary is offered as a
  download only. The commit subject heads the page when the base names a commit.
  A request with neither an id nor a file name is a bad request; a file name
  absent from the base is not found.

  Background:
    Given the blob base is commit "Initial import"

  Scenario: a text file is decoded into numbered lines
    Given the blob "readme" at "README" contains text "hello\nworld"
    When I assemble the blob at path "README"
    Then the blob displays as text
    And the blob has 2 lines
    And blob line 1 is "hello"
    And blob line 2 is "world"
    And the blob shows the commit subject "Initial import"

  Scenario: an empty file has no lines
    Given the blob "empty" at "empty.txt" contains text ""
    When I assemble the blob at path "empty.txt"
    Then the blob displays as text
    And the blob has no lines

  Scenario: a non-UTF8 file is decoded through the latin1 fallback
    Given the blob "latin" at "latin1.txt" contains bytes "48 e9 6c 6c 6f"
    When I assemble the blob at path "latin1.txt"
    Then the blob displays as text
    And blob line 1 is "Héllo"

  Scenario: a binary file is offered as a download, not inlined
    Given the blob "data" at "data.bin" contains bytes "00 01 02"
    When I assemble the blob at path "data.bin"
    Then the blob displays as binary
    And the blob has no lines

  Scenario: an image file is marked for inline display
    Given the blob "logo" at "logo.png" contains bytes "89 50 00 47"
    When I assemble the blob at path "logo.png"
    Then the blob displays as an image
    And the blob has no lines

  Scenario: a file can be shown by its id
    Given the blob "readme" at "README" contains text "by id"
    When I assemble the blob of id "readme"
    Then the blob displays as text
    And blob line 1 is "by id"

  Scenario: a file shown by id alone has no commit subject
    Given the blob "readme" at "README" contains text "loose"
    When I assemble the blob of id "readme"
    Then the blob shows no commit subject

  Scenario: a request with neither id nor file name is a bad request
    When I assemble the blob with neither id nor path
    Then assembling the blob fails as invalid

  Scenario: a path absent from the base is not found
    When I assemble the blob at path "ghost"
    Then assembling the blob fails as not found

  Scenario: a blob shown by id with an unresolvable base still renders, without a subject
    Given the blob "readme" at "README" contains text "by id"
    When I assemble the blob of id "readme" with base "nonesuch"
    Then the blob displays as text
    And blob line 1 is "by id"
    And the blob shows no commit subject
