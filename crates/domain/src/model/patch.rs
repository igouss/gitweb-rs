//! Unified diff (patch) text formatting.
//!
//! The textual unified diff is git's own patch format, the bytes `git diff-tree
//! -p` emits and gitweb's `commitdiff_plain` / `blobdiff_plain` / `patch` /
//! `patches` endpoints stream verbatim. gitweb never builds this format itself —
//! it only re-decorates git's output (`format_git_diff_header_line`,
//! `format_extended_diff_header_line`, `format_diff_from_to_header`) — so the
//! format is the source of truth, and it lives here as a pure rule.
//!
//! The split is deliberate: this module owns the *format* (which extended
//! headers a status implies, where `/dev/null` goes, how an `@@` range is
//! written, the `\ No newline at end of file` marker), while the gix adapter
//! owns the *algorithm* (which lines changed) and blob IO and feeds the
//! structured [`Hunk`]s in.
//!
//! Object ids are rendered in full (40/64 hex), matching `git diff-tree
//! --full-index` and keeping the output deterministic; abbreviating the `index`
//! line the way bare `diff-tree -p` does is an endpoint concern, not a format
//! rule.

use crate::model::change::{ChangeKind, ChangeStatus};
use crate::model::file_mode::FileMode;
use crate::model::object_id::ObjectId;

/// Whether a hunk line is shared context, an addition, or a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkLineKind {
    /// A line present in both sides, shown for context (` ` prefix).
    Context,
    /// A line only in the new side (`+` prefix).
    Addition,
    /// A line only in the old side (`-` prefix).
    Deletion,
}

impl HunkLineKind {
    /// The single-character prefix git writes before the line text.
    #[must_use]
    pub fn prefix(self) -> char {
        match self {
            HunkLineKind::Context => ' ',
            HunkLineKind::Addition => '+',
            HunkLineKind::Deletion => '-',
        }
    }
}

/// One line within a hunk.
///
/// `text` is the line content without git's `+`/`-`/space prefix and without
/// the trailing newline. `no_newline_after` records git's `\ No newline at end
/// of file` marker, which follows the last line of a side that does not end in a
/// newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkLine {
    kind: HunkLineKind,
    text: String,
    no_newline_after: bool,
}

impl HunkLine {
    /// A context line shared by both sides.
    #[must_use]
    pub fn context(text: impl Into<String>) -> Self {
        Self::new(HunkLineKind::Context, text)
    }

    /// An added line.
    #[must_use]
    pub fn addition(text: impl Into<String>) -> Self {
        Self::new(HunkLineKind::Addition, text)
    }

    /// A removed line.
    #[must_use]
    pub fn deletion(text: impl Into<String>) -> Self {
        Self::new(HunkLineKind::Deletion, text)
    }

    fn new(kind: HunkLineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            no_newline_after: false,
        }
    }

    /// Marks this line as the last of a side that lacks a trailing newline, so
    /// the `\ No newline at end of file` marker is emitted after it.
    #[must_use]
    pub fn without_trailing_newline(mut self) -> Self {
        self.no_newline_after = true;
        self
    }

    /// Appends the prefixed line, then git's `\ No newline at end of file`
    /// marker if this line ends a side that has no trailing newline.
    fn write_to(&self, out: &mut String) {
        out.push(self.kind.prefix());
        out.push_str(&self.text);
        out.push('\n');
        if self.no_newline_after {
            out.push_str("\\ No newline at end of file\n");
        }
    }
}

/// A single `@@` hunk: a contiguous run of context, additions, and removals,
/// positioned by 1-based start lines and run lengths on each side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    before_start: u32,
    before_len: u32,
    after_start: u32,
    after_len: u32,
    section_heading: Option<String>,
    lines: Vec<HunkLine>,
}

impl Hunk {
    /// Assembles a hunk from its line-number window and its lines.
    #[must_use]
    pub fn new(
        before_start: u32,
        before_len: u32,
        after_start: u32,
        after_len: u32,
        lines: Vec<HunkLine>,
    ) -> Self {
        Self {
            before_start,
            before_len,
            after_start,
            after_len,
            section_heading: None,
            lines,
        }
    }

    /// Attaches the section heading git appends after the closing `@@` (the
    /// enclosing function/section a hunk falls in).
    #[must_use]
    pub fn with_section_heading(mut self, heading: impl Into<String>) -> Self {
        self.section_heading = Some(heading.into());
        self
    }

    /// Appends the `@@ -bs,bl +as,al @@` header and the hunk's lines.
    fn write_to(&self, out: &mut String) {
        let before: String = fmt_range(self.before_start, self.before_len);
        let after: String = fmt_range(self.after_start, self.after_len);
        let heading: String = match &self.section_heading {
            Some(text) => format!(" {text}"),
            None => String::new(),
        };
        out.push_str(&format!("@@ -{before} +{after} @@{heading}\n"));
        for line in &self.lines {
            line.write_to(out);
        }
    }
}

/// Formats one side of an `@@` header. git omits the `,count` when the run is a
/// single line (`@@ -1 +1 @@`), and keeps it otherwise — including the `,0` of
/// an empty side (`@@ -0,0 +1,2 @@` for a file created from nothing).
fn fmt_range(start: u32, len: u32) -> String {
    if len == 1 {
        format!("{start}")
    } else {
        format!("{start},{len}")
    }
}

/// A file patch's body: a binary notice, or a sequence of text hunks (possibly
/// empty, for a pure rename or mode change that touches no content).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// The blob is binary; git prints a one-line notice instead of hunks.
    Binary,
    /// The blob is text; these are its hunks.
    Text(Vec<Hunk>),
}

/// One file's worth of patch: its change status, the from/to modes, ids and
/// paths, and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePatch {
    status: ChangeStatus,
    from_mode: FileMode,
    to_mode: FileMode,
    from_oid: ObjectId,
    to_oid: ObjectId,
    from_path: String,
    to_path: String,
    content: FileContent,
}

impl FilePatch {
    /// Assembles one file patch.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: ChangeStatus,
        from_mode: FileMode,
        to_mode: FileMode,
        from_oid: ObjectId,
        to_oid: ObjectId,
        from_path: impl Into<String>,
        to_path: impl Into<String>,
        content: FileContent,
    ) -> Self {
        Self {
            status,
            from_mode,
            to_mode,
            from_oid,
            to_oid,
            from_path: from_path.into(),
            to_path: to_path.into(),
            content,
        }
    }

    /// Renders this file patch as git's unified-diff text, each line newline
    /// terminated, with full 40/64-hex `index` ids (`--full-index`).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out: String = String::new();
        self.write_to(&mut out, None);
        out
    }

    /// Appends this file patch to `out`. Shared by [`Patch::render`] so a
    /// multi-file patch is one streamed concatenation, exactly as git emits it.
    /// `abbrev` truncates the `index` ids to that many hex characters (bare
    /// `diff-tree -p`'s default short form); `None` keeps them full.
    fn write_to(&self, out: &mut String, abbrev: Option<usize>) {
        let kind: ChangeKind = self.status.kind();
        let is_create: bool = matches!(kind, ChangeKind::Added);
        let is_delete: bool = matches!(kind, ChangeKind::Deleted);

        out.push_str(&format!(
            "diff --git a/{} b/{}\n",
            self.from_path, self.to_path
        ));
        self.write_extended_headers(out, kind, is_create, is_delete, abbrev);
        self.write_body(out, is_create, is_delete);
    }

    /// The extended headers between the `diff --git` line and the body: the
    /// mode-change / creation / deletion announcements, the rename/copy block,
    /// and the `index` line, in git's order.
    fn write_extended_headers(
        &self,
        out: &mut String,
        kind: ChangeKind,
        is_create: bool,
        is_delete: bool,
        abbrev: Option<usize>,
    ) {
        // A mode change is announced only when both sides exist; creation and
        // deletion carry the mode in their own header instead.
        if !is_create && !is_delete && self.from_mode != self.to_mode {
            out.push_str(&format!("old mode {}\n", self.from_mode.octal()));
            out.push_str(&format!("new mode {}\n", self.to_mode.octal()));
        }
        if is_create {
            out.push_str(&format!("new file mode {}\n", self.to_mode.octal()));
        }
        if is_delete {
            out.push_str(&format!("deleted file mode {}\n", self.from_mode.octal()));
        }
        match kind {
            ChangeKind::Renamed => {
                out.push_str(&format!("similarity index {}%\n", self.status.similarity()));
                out.push_str(&format!("rename from {}\n", self.from_path));
                out.push_str(&format!("rename to {}\n", self.to_path));
            }
            ChangeKind::Copied => {
                out.push_str(&format!("similarity index {}%\n", self.status.similarity()));
                out.push_str(&format!("copy from {}\n", self.from_path));
                out.push_str(&format!("copy to {}\n", self.to_path));
            }
            _ => {}
        }
        // The `index` line appears only when the blob identity actually
        // changed; a pure mode change or an exact rename/copy leaves both sides
        // pointing at the same object and git omits it. The trailing mode rides
        // on the line only when both sides share one mode (otherwise the modes
        // are shown separately, and creation/deletion show them in their own
        // header).
        if self.from_oid != self.to_oid {
            let trailing_mode: String =
                if !is_create && !is_delete && self.from_mode == self.to_mode {
                    format!(" {}", self.to_mode.octal())
                } else {
                    String::new()
                };
            out.push_str(&format!(
                "index {}..{}{}\n",
                abbrev_hex(self.from_oid.as_str(), abbrev),
                abbrev_hex(self.to_oid.as_str(), abbrev),
                trailing_mode
            ));
        }
    }

    /// The body: a one-line binary notice, or the `--- `/`+++ ` lines followed
    /// by the hunks. A content-less change (exact rename, pure mode change)
    /// has no hunks and so emits no `--- `/`+++ ` at all.
    fn write_body(&self, out: &mut String, is_create: bool, is_delete: bool) {
        let from_label: String = if is_create {
            "/dev/null".to_owned()
        } else {
            format!("a/{}", self.from_path)
        };
        let to_label: String = if is_delete {
            "/dev/null".to_owned()
        } else {
            format!("b/{}", self.to_path)
        };
        match &self.content {
            FileContent::Binary => {
                out.push_str(&format!(
                    "Binary files {from_label} and {to_label} differ\n"
                ));
            }
            FileContent::Text(hunks) => {
                if hunks.is_empty() {
                    return;
                }
                out.push_str(&format!("--- {from_label}\n"));
                out.push_str(&format!("+++ {to_label}\n"));
                for hunk in hunks {
                    hunk.write_to(out);
                }
            }
        }
    }
}

/// A whole patch: the file patches of one diff, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Patch {
    files: Vec<FilePatch>,
}

impl Patch {
    /// Builds a patch from its file patches.
    #[must_use]
    pub fn new(files: Vec<FilePatch>) -> Self {
        Self { files }
    }

    /// Renders the whole patch: each file patch's text, concatenated in order
    /// the way git streams them, with full `index` ids (`--full-index`).
    #[must_use]
    pub fn render(&self) -> String {
        self.render_with(None)
    }

    /// Renders the whole patch with the `index` ids abbreviated to `abbrev_len`
    /// hex characters — the short form bare `git diff-tree -p` emits (without
    /// `--full-index`), which gitweb's `commitdiff_plain` streams. The null
    /// (all-zero) id of a created or deleted side abbreviates to the same width,
    /// so `index 0000000..f971a5e` comes out exactly as git writes it.
    ///
    /// The width is uniform across every id, matching git for any repository
    /// whose objects are unique at that length (git only extends an individual
    /// id past the default on the rare prefix collision); the caller passes the
    /// repository's default abbreviation, which the adapter reports.
    #[must_use]
    pub fn render_abbreviated(&self, abbrev_len: usize) -> String {
        self.render_with(Some(abbrev_len))
    }

    /// Concatenates every file patch, optionally abbreviating the `index` ids.
    fn render_with(&self, abbrev: Option<usize>) -> String {
        let mut out: String = String::new();
        for file in &self.files {
            file.write_to(&mut out, abbrev);
        }
        out
    }
}

/// The `index`-line form of an object id: the full hex, or its first
/// `len` characters when an abbreviation is requested. The id is ASCII hex, so a
/// byte-prefix is a valid character boundary; an over-long `len` is clamped to
/// the id's own length.
fn abbrev_hex(full: &str, abbrev: Option<usize>) -> &str {
    match abbrev {
        Some(len) => &full[..len.min(full.len())],
        None => full,
    }
}
