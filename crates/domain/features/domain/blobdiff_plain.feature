Feature: The blobdiff_plain body format
  gitweb's `blobdiff_plain` is the single-file analogue of `commitdiff_plain`:
  a format-stable endpoint streaming the raw unified diff of one file. Its
  framing is thin — a single `X-Git-Url:` line carrying the request's own self
  link, a blank line, then the bare single-file patch. There is no mailbox
  `From:` / `Subject:` header; that header is what would break a diff viewer's
  parser, and `blobdiff_plain` deliberately omits it.

  The patch body arrives already rendered (abbreviated `index` ids, decoded),
  so this rule only frames it. The self link is supplied at render time — the
  domain never builds a URL.

  Scenario: The body is the X-Git-Url line, a blank line, then the patch
    Given a blobdiff_plain whose patch body is "diff --git a/x b/x"
    When I render the blobdiff_plain at "http://localhost?p=repo.git;a=blobdiff_plain;hb=feed;hpb=cafe;f=x"
    Then the blobdiff_plain body is:
      """
      X-Git-Url: http://localhost?p=repo.git;a=blobdiff_plain;hb=feed;hpb=cafe;f=x

      diff --git a/x b/x
      """

  Scenario: A multi-line patch body rides verbatim after the blank line
    Given a blobdiff_plain whose patch body is:
      """
      diff --git a/x b/x
      index 1111111..2222222 100644
      --- a/x
      +++ b/x
      @@ -1 +1 @@
      -old
      +new
      """
    When I render the blobdiff_plain at "http://localhost?p=repo.git;a=blobdiff_plain;hb=feed;hpb=cafe;f=x"
    Then the blobdiff_plain body is:
      """
      X-Git-Url: http://localhost?p=repo.git;a=blobdiff_plain;hb=feed;hpb=cafe;f=x

      diff --git a/x b/x
      index 1111111..2222222 100644
      --- a/x
      +++ b/x
      @@ -1 +1 @@
      -old
      +new
      """
