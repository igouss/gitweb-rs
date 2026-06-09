//! The git format-patch mailbox stream gitweb's `patch` / `patches` stream.
//!
//! gitweb's `patch` (one commit) and `patches` (a commit range) actions stream
//! `git format-patch --stdout` verbatim, so this is git's mailbox format, not
//! gitweb's, and its bytes must match. Each commit becomes one `git am`-able
//! mail:
//!
//! ```text
//! From <40-hex commit id> Mon Sep 17 00:00:00 2001   # git's fixed magic date
//! From: <author ident>
//! Date: <author date, RFC-2822, the commit's own zone>
//! Subject: [PATCH] <subject>                         # [PATCH i/N] when numbered
//!                                                    # (one blank line)
//! <body lines>                                       # the message past the subject
//! ---
//! <diffstat>                                         # git's --stat --summary
//!                                                    # (one blank line)
//! <diff>                                             # the unified diff (abbrev index)
//! -- <SP>                                            # the mail signature delimiter
//! <git version>
//! ```
//!
//! The mails are joined by a blank line and the stream ends with one — git's
//! `blocks.join("\n\n") + "\n"`. The trailing `git version` is volatile (not part
//! of the format and not byte-stable across machines), so the boundary supplies
//! it; the golden pins it the way the feed pins its generator version.
//!
//! This entity only frames: the [`Diffstat`] and the diff body arrive already
//! rendered (the use case builds the stat from the patch and renders the diff
//! with abbreviated `index` ids), the subject and body already split out of the
//! message, the author date already a [`Timestamp`]. The single-line ASCII
//! subject the corpus carries needs no RFC-2047 encoding or header folding; git's
//! handling of a non-ASCII or over-long subject is a deeper edge left for when a
//! view needs it.

use crate::model::diffstat::Diffstat;
use crate::model::timestamp::Timestamp;

/// git's fixed "magic" date on the `From <id> …` mbox separator line — a
/// constant, never the commit's real date.
const MAGIC_DATE: &str = "Mon Sep 17 00:00:00 2001";

/// One commit rendered as a format-patch mail, minus the trailing signature
/// version the stream appends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchEntry {
    /// The commit's full 40/64-hex object id — the `From <id>` separator line.
    commit_id: String,
    /// The author identity (`Name <email>`), gitweb's `$co{'author'}`.
    author: String,
    /// The author timestamp; its RFC-2822 local form feeds the `Date:` line.
    date: Timestamp,
    /// The subject (the message's first line), the `Subject:` line's text.
    subject: String,
    /// The patch's 1-based position and the total count, when the stream is
    /// numbered (`patches`, git's `-n`): `Some((i, n))` writes `[PATCH i/N]`,
    /// `None` writes a bare `[PATCH]` (`patch`, the single form).
    number: Option<(usize, usize)>,
    /// The message body past the subject, one entry per line (printed verbatim).
    body: Vec<String>,
    /// The diffstat block (`--stat --summary`).
    diffstat: Diffstat,
    /// The unified diff body, already rendered with abbreviated `index` ids.
    diff_body: String,
}

impl PatchEntry {
    /// Assembles a patch mail from its parts.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        commit_id: impl Into<String>,
        author: impl Into<String>,
        date: Timestamp,
        subject: impl Into<String>,
        number: Option<(usize, usize)>,
        body: Vec<String>,
        diffstat: Diffstat,
        diff_body: impl Into<String>,
    ) -> Self {
        Self {
            commit_id: commit_id.into(),
            author: author.into(),
            date,
            subject: subject.into(),
            number,
            body,
            diffstat,
            diff_body: diff_body.into(),
        }
    }

    /// The `Subject:` prefix: `[PATCH]` for the single form, `[PATCH i/N]` when
    /// the stream is numbered.
    fn subject_prefix(&self) -> String {
        match self.number {
            Some((index, total)) => format!("[PATCH {index}/{total}]"),
            None => "[PATCH]".to_owned(),
        }
    }

    /// Renders this mail up to and including the signature `version` line — no
    /// trailing blank (the stream's join supplies the separators).
    fn render(&self, version: &str) -> String {
        let mut out: String = String::new();
        out.push_str(&format!("From {} {MAGIC_DATE}\n", self.commit_id));
        out.push_str(&format!("From: {}\n", self.author));
        out.push_str(&format!("Date: {}\n", self.date.rfc2822_local()));
        out.push_str(&format!(
            "Subject: {} {}\n",
            self.subject_prefix(),
            self.subject
        ));
        out.push('\n');
        for line in &self.body {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("---\n");
        out.push_str(&self.diffstat.render());
        out.push('\n');
        out.push_str(&self.diff_body);
        out.push_str("-- \n");
        out.push_str(version);
        out.push('\n');
        out
    }
}

/// A format-patch stream: one or more [`PatchEntry`] mails plus the git version
/// stamped on every signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatPatch {
    entries: Vec<PatchEntry>,
    version: String,
}

impl FormatPatch {
    /// Builds a stream from its mails and the git version its signatures carry.
    #[must_use]
    pub fn new(entries: Vec<PatchEntry>, version: impl Into<String>) -> Self {
        Self {
            entries,
            version: version.into(),
        }
    }

    /// Renders the whole stream: each mail, joined by a blank line, the stream
    /// ending with one — git's `blocks.join("\n\n") + "\n"`. A single mail comes
    /// out followed by its one trailing blank line.
    #[must_use]
    pub fn render(&self) -> String {
        let blocks: Vec<String> = self
            .entries
            .iter()
            .map(|entry: &PatchEntry| entry.render(&self.version))
            .collect();
        let mut out: String = blocks.join("\n\n");
        out.push('\n');
        out
    }
}
