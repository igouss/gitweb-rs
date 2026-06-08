Feature: The commitdiff_plain body format
  gitweb's `commitdiff_plain` is a format-stable endpoint: it frames git's raw
  diff in a mailbox-style header so the result reads as a `git am`-able patch.
  The header is `From:` / `Date:` (RFC-2822 with the local timezone in parens)
  / `Subject:`, an optional `X-Git-Tag:` when the commit is tag-named, and
  `X-Git-Url:` carrying the request's own self link; then a blank line, the
  commit comment, a `---` separator, a blank line, and finally the patch. For
  the root / merge form, `git diff-tree` prefixes the patch with the bare commit
  hash; the two-tree form has no such line.

  The patch body arrives already rendered (abbreviated `index` ids, decoded),
  so this rule only frames it. The self link is supplied at render time — the
  domain never builds a URL.

  Scenario: A root commit frames the header, comment, hash line and patch
    Given a commitdiff_plain by "Ada Lovelace <ada@example.com>" titled "corpus root"
    And its comment line is "corpus root"
    And it carries the commit-id line "331d8105735f3b7bd6256afbd609616b78475faa"
    And its patch body is "diff --git a/x b/x"
    When I render the commitdiff_plain at "http://localhost?p=repo.git;a=commitdiff_plain;h=331d8105735f3b7bd6256afbd609616b78475faa"
    Then the commitdiff_plain body is:
      """
      From: Ada Lovelace <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000 (+0000)
      Subject: corpus root
      X-Git-Url: http://localhost?p=repo.git;a=commitdiff_plain;h=331d8105735f3b7bd6256afbd609616b78475faa

      corpus root
      ---

      331d8105735f3b7bd6256afbd609616b78475faa
      diff --git a/x b/x
      """

  Scenario: A two-tree commit has no bare commit-id line before the patch
    Given a commitdiff_plain by "Ada Lovelace <ada@example.com>" titled "second"
    And its comment line is "second"
    And its patch body is "diff --git a/x b/x"
    When I render the commitdiff_plain at "http://localhost?p=repo.git;a=commitdiff_plain;h=cafe"
    Then the commitdiff_plain body is:
      """
      From: Ada Lovelace <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000 (+0000)
      Subject: second
      X-Git-Url: http://localhost?p=repo.git;a=commitdiff_plain;h=cafe

      second
      ---

      diff --git a/x b/x
      """

  Scenario: A tag-named commit gains the X-Git-Tag line
    Given a commitdiff_plain by "Ada Lovelace <ada@example.com>" titled "release"
    And it is tag-named "v1.0"
    And its comment line is "release"
    And its patch body is "diff --git a/x b/x"
    When I render the commitdiff_plain at "http://localhost?p=repo.git;a=commitdiff_plain;h=cafe"
    Then the commitdiff_plain body is:
      """
      From: Ada Lovelace <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000 (+0000)
      Subject: release
      X-Git-Tag: v1.0
      X-Git-Url: http://localhost?p=repo.git;a=commitdiff_plain;h=cafe

      release
      ---

      diff --git a/x b/x
      """

  Scenario: A multi-line comment prints every line in order
    Given a commitdiff_plain by "Ada Lovelace <ada@example.com>" titled "subject line"
    And its comment line is "subject line"
    And its comment line is ""
    And its comment line is "body paragraph"
    And its patch body is "diff --git a/x b/x"
    When I render the commitdiff_plain at "http://localhost?p=repo.git;a=commitdiff_plain;h=cafe"
    Then the commitdiff_plain body is:
      """
      From: Ada Lovelace <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000 (+0000)
      Subject: subject line
      X-Git-Url: http://localhost?p=repo.git;a=commitdiff_plain;h=cafe

      subject line

      body paragraph
      ---

      diff --git a/x b/x
      """
