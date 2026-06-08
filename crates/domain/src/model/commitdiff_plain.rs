//! The `commitdiff_plain` body format (gitweb's `git_commitdiff -format plain`).
//!
//! A format-stable endpoint: other tools (`git am`) parse it, so its bytes must
//! match the original. gitweb prints a mailbox-style header, a blank line, the
//! commit's comment, a `---` separator, then the raw `git diff-tree -p` output:
//!
//! ```text
//! From: <author ident>
//! Date: <rfc2822> (<tz_local>)
//! Subject: <commit title>
//! X-Git-Tag: <tagname>            # only when the commit is tag-named
//! X-Git-Url: <self_url>
//!                                 # (one blank line)
//! <each comment line>
//! ---
//!                                 # (one blank line)
//! <commit hash>                   # only for the --root/--cc single-commit form
//! <patch>
//! ```
//!
//! The header lines, the comment, and the patch are the *what*; the `self_url`
//! is the *where* (the boundary builds it, so [`CommitdiffPlain::render`] takes
//! it as an argument rather than constructing a URL). The patch body is already
//! rendered with abbreviated `index` ids — bare `diff-tree -p`'s short form —
//! and decoded the way gitweb's `:utf8` output layer encodes git's raw bytes,
//! so this module only frames it; it never re-encodes.
//!
//! The bare commit-hash line that precedes the patch is `git diff-tree`'s own
//! output when it is handed a single commit-ish (gitweb's `--root` or `--cc`),
//! and is absent for the two-tree (explicit/single-parent) form. The use case
//! decides which by whether it diffs against the empty tree, and passes the
//! hash here only when the line is due.

use crate::model::timestamp::Timestamp;

/// A rendered `commitdiff_plain` response body, minus the `self_url` the
/// boundary supplies at render time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitdiffPlain {
    /// The author identity (`Name <email>`), as gitweb's `$co{'author'}`.
    author: String,
    /// The author timestamp — its RFC-2822 form and timezone token feed the
    /// `Date:` line.
    date: Timestamp,
    /// The commit title, gitweb's `$co{'title'}` (chopped first message line).
    subject: String,
    /// The tag name for `X-Git-Tag`, when the commit is tag-named; `None` omits
    /// the line.
    tag: Option<String>,
    /// The commit's comment, one entry per message line (printed verbatim).
    comment: Vec<String>,
    /// The bare commit-hash line `git diff-tree` prints for the single-commit
    /// (`--root`/`--cc`) form; `None` for the two-tree form, which has none.
    commit_id: Option<String>,
    /// The patch body: the abbreviated-`index` `diff-tree -p` text, already
    /// decoded, that the framing wraps verbatim.
    patch_body: String,
}

impl CommitdiffPlain {
    /// Frames a `commitdiff_plain` body from its parts.
    #[must_use]
    pub fn new(
        author: impl Into<String>,
        date: Timestamp,
        subject: impl Into<String>,
        tag: Option<String>,
        comment: Vec<String>,
        commit_id: Option<String>,
        patch_body: impl Into<String>,
    ) -> Self {
        Self {
            author: author.into(),
            date,
            subject: subject.into(),
            tag,
            comment,
            commit_id,
            patch_body: patch_body.into(),
        }
    }

    /// Renders the full response body, placing `self_url` (the boundary's
    /// absolute self-link) on the `X-Git-Url` line. Every line is newline
    /// terminated, matching gitweb's `print "...\n"` per field.
    #[must_use]
    pub fn render(&self, self_url: &str) -> String {
        let mut out: String = String::new();
        out.push_str(&format!("From: {}\n", self.author));
        out.push_str(&format!(
            "Date: {} ({})\n",
            self.date.rfc2822(),
            self.date.timezone()
        ));
        out.push_str(&format!("Subject: {}\n", self.subject));
        if let Some(tag) = &self.tag {
            out.push_str(&format!("X-Git-Tag: {tag}\n"));
        }
        out.push_str(&format!("X-Git-Url: {self_url}\n"));
        out.push('\n');
        for line in &self.comment {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("---\n");
        out.push('\n');
        if let Some(id) = &self.commit_id {
            out.push_str(id);
            out.push('\n');
        }
        out.push_str(&self.patch_body);
        out
    }
}
