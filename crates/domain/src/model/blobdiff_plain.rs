//! The `blobdiff_plain` body format (gitweb's `git_blobdiff('plain')`).
//!
//! The single-file analogue of [`CommitdiffPlain`](crate::model::commitdiff_plain):
//! the raw unified diff of *one* file between two revisions. It is a
//! format-stable endpoint — its bytes must match the original `gitweb.perl`, so
//! other tools can apply it — but its framing is far thinner than
//! `commitdiff_plain`'s mailbox header. gitweb prints a single `X-Git-Url` line
//! carrying the request's own self link, a blank line, then the bare
//! `git diff-tree -p` output for the one path:
//!
//! ```text
//! X-Git-Url: <self_url>
//!                                 # (one blank line)
//! <single-file patch>
//! ```
//!
//! The patch body arrives already rendered with abbreviated `index` ids (the
//! short form bare `git diff-tree -p` writes — `blobdiff_plain` does *not* pass
//! `--full-index`) and decoded the way gitweb's `:utf8` output layer encodes
//! git's raw bytes, so this rule only frames it; it never re-encodes. The
//! `self_url` is the *where* — the boundary builds it, so [`BlobdiffPlain::render`]
//! takes it as an argument rather than constructing a URL, exactly as
//! `commitdiff_plain` does.

/// A rendered `blobdiff_plain` response body, minus the `self_url` the boundary
/// supplies at render time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobdiffPlain {
    /// The single-file patch body: the abbreviated-`index` `diff-tree -p` text
    /// for one path, already decoded, that the framing wraps verbatim.
    patch_body: String,
}

impl BlobdiffPlain {
    /// Frames a `blobdiff_plain` body from its single-file patch text.
    #[must_use]
    pub fn new(patch_body: impl Into<String>) -> Self {
        Self {
            patch_body: patch_body.into(),
        }
    }

    /// Renders the full response body, placing `self_url` (the boundary's
    /// absolute self-link) on the `X-Git-Url` line, then a blank line, then the
    /// patch — matching gitweb's `print "X-Git-Url: " . self_url . "\n\n"`
    /// followed by the streamed diff.
    #[must_use]
    pub fn render(&self, self_url: &str) -> String {
        format!("X-Git-Url: {self_url}\n\n{}", self.patch_body)
    }
}
