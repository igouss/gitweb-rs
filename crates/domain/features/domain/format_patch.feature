Feature: The git format-patch mailbox stream
  gitweb's `patch` and `patches` actions stream `git format-patch --stdout`
  verbatim, so this is git's mailbox format. Each commit is one `git am`-able
  mail: a `From <id> Mon Sep 17 00:00:00 2001` separator with git's fixed magic
  date, the `From:` / `Date:` / `Subject:` headers, a blank line, the message
  body, a `---` line, the diffstat, a blank line, the diff, and a `-- ` / git
  version signature. Mails are joined by a blank line and the stream ends with
  one.

  This rule only frames: the diffstat and the diff body arrive already rendered,
  the subject and body already split off the message. The volatile git version
  on the signature is supplied at render time.

  Scenario: A single commit frames one mail with the [PATCH] subject
    Given a patch mail for commit "b0797526e38bfa900c9cb532bfebc02ac8b56c9f" by "Ada Lovelace <ada@example.com>" at epoch 1700000000 zone "+0000"
    And the patch subject is "Add the analytical engine"
    And the patch body line is "The first program."
    And a created file "engine.txt" mode "100644" with 2 added 0 deleted
    And the patch diff body is:
      """
      diff --git a/engine.txt b/engine.txt
      new file mode 100644
      index 0000000..66a52ee
      --- /dev/null
      +++ b/engine.txt
      @@ -0,0 +1,2 @@
      +first
      +second
      """
    And the mail is complete
    When I render the format-patch stream with version "2.54.0"
    Then the format-patch stream is:
      """
      From b0797526e38bfa900c9cb532bfebc02ac8b56c9f Mon Sep 17 00:00:00 2001
      From: Ada Lovelace <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000
      Subject: [PATCH] Add the analytical engine

      The first program.
      ---
       engine.txt | 2 ++
       1 file changed, 2 insertions(+)
       create mode 100644 engine.txt

      diff --git a/engine.txt b/engine.txt
      new file mode 100644
      index 0000000..66a52ee
      --- /dev/null
      +++ b/engine.txt
      @@ -0,0 +1,2 @@
      +first
      +second
      --
      2.54.0
      """

  Scenario: A numbered range joins its mails and numbers each subject
    Given a patch mail for commit "1111111111111111111111111111111111111111" by "Ada <ada@example.com>" at epoch 1700000000 zone "+0000"
    And the patch subject is "first"
    And it is patch 1 of 2
    And a created file "a.txt" mode "100644" with 1 added 0 deleted
    And the patch diff body is:
      """
      diff --git a/a.txt b/a.txt
      new file mode 100644
      index 0000000..0000001
      --- /dev/null
      +++ b/a.txt
      @@ -0,0 +1 @@
      +A
      """
    And the mail is complete
    And a patch mail for commit "2222222222222222222222222222222222222222" by "Ada <ada@example.com>" at epoch 1700000000 zone "+0000"
    And the patch subject is "second"
    And it is patch 2 of 2
    And a created file "b.txt" mode "100644" with 1 added 0 deleted
    And the patch diff body is:
      """
      diff --git a/b.txt b/b.txt
      new file mode 100644
      index 0000000..0000002
      --- /dev/null
      +++ b/b.txt
      @@ -0,0 +1 @@
      +B
      """
    And the mail is complete
    When I render the format-patch stream with version "2.54.0"
    Then the format-patch stream is:
      """
      From 1111111111111111111111111111111111111111 Mon Sep 17 00:00:00 2001
      From: Ada <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000
      Subject: [PATCH 1/2] first

      ---
       a.txt | 1 +
       1 file changed, 1 insertion(+)
       create mode 100644 a.txt

      diff --git a/a.txt b/a.txt
      new file mode 100644
      index 0000000..0000001
      --- /dev/null
      +++ b/a.txt
      @@ -0,0 +1 @@
      +A
      --
      2.54.0


      From 2222222222222222222222222222222222222222 Mon Sep 17 00:00:00 2001
      From: Ada <ada@example.com>
      Date: Tue, 14 Nov 2023 22:13:20 +0000
      Subject: [PATCH 2/2] second

      ---
       b.txt | 1 +
       1 file changed, 1 insertion(+)
       create mode 100644 b.txt

      diff --git a/b.txt b/b.txt
      new file mode 100644
      index 0000000..0000002
      --- /dev/null
      +++ b/b.txt
      @@ -0,0 +1 @@
      +B
      --
      2.54.0
      """
