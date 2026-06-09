//! The diffstat block git format-patch prints between `---` and the patch.
//!
//! This is git's `--stat --summary` output — the `show_stats` and `diff_summary`
//! routines in git's `diff.c` — not gitweb code: gitweb's `patch` / `patches`
//! actions stream `git format-patch` verbatim, so the diffstat's bytes are git's
//! and must be reproduced to the byte. It is two stacked blocks:
//!
//! ```text
//!  src/foo.rs | 12 +++++++-----      # the --stat lines: one per file,
//!  src/bar.rs |  3 ++-               #   name | total <graph of +/->,
//!  2 files changed, 9 insertions(+), 6 deletions(-)   # the summary line,
//!  create mode 100644 src/bar.rs    # then the --summary lines:
//!  rename src/old.rs => src/foo.rs (88%)              #   create/delete/rename/
//!  mode change 100644 => 100755 run.sh                #   copy/mode-change.
//! ```
//!
//! The hard part is the `--stat` column maths, reproduced faithfully from
//! `show_stats`: the name column is padded to the widest name; the change count
//! is right-justified to the widest count's digits; the `+`/`-` graph is scaled
//! to a fixed width when the changes would overflow it, with `scale_linear`
//! keeping at least one bar per changed file. Binary files show `Bin <old> ->
//! <new> bytes` instead of a graph.
//!
//! That width is `format-patch`'s, not the terminal's: `cmd_format_patch` pins
//! `stat_width` to `MAIL_DEFAULT_WRAP` (72) so a mailed patch wraps the same on
//! every machine — terminal-independent, unlike `git show --stat`'s 80-column
//! `term_columns()`. Reproducing it here means the golden is byte-stable no
//! matter the capturing host's `COLUMNS` or tty.
//!
//! The split from [`crate::model::patch`] is deliberate: `patch` owns the unified
//! diff *body*, this owns the *stat*. Both are derived from the same structured
//! file changes; the use case feeds this the per-file magnitudes (line counts for
//! text, byte sizes for binary) that only it, with the repository, can total.

use crate::model::change::{ChangeKind, ChangeStatus};
use crate::model::file_mode::FileMode;

/// git's `MAIL_DEFAULT_WRAP`: the fixed width `format-patch` pins its diffstat to
/// (`cmd_format_patch`'s `stat_width = MAIL_DEFAULT_WRAP`), independent of the
/// terminal — so a mailed patch, and this golden, wrap identically everywhere.
const STAT_WIDTH: i64 = 72;

/// The magnitude of one file's change, the way the stat reports it: a text file
/// by its added/deleted line counts, a binary file by its old/new byte sizes
/// (git's `Bin <old> -> <new> bytes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatChange {
    /// A text file: lines added and lines deleted.
    Text {
        /// Lines added (`+`).
        added: u32,
        /// Lines deleted (`-`).
        deleted: u32,
    },
    /// A binary file: old and new blob sizes in bytes.
    Binary {
        /// The pre-image size, git's `Bin <old> -> …`.
        old_size: u64,
        /// The post-image size, git's `… -> <new> bytes`.
        new_size: u64,
    },
}

/// One file's entry in a diffstat: its change status, the from/to modes and
/// paths (which decide its `--summary` line and its stat name column), and the
/// magnitude of its change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffstatEntry {
    status: ChangeStatus,
    from_mode: FileMode,
    to_mode: FileMode,
    from_path: String,
    to_path: String,
    change: StatChange,
}

impl DiffstatEntry {
    /// Assembles a diffstat entry from its parts.
    #[must_use]
    pub fn new(
        status: ChangeStatus,
        from_mode: FileMode,
        to_mode: FileMode,
        from_path: impl Into<String>,
        to_path: impl Into<String>,
        change: StatChange,
    ) -> Self {
        Self {
            status,
            from_mode,
            to_mode,
            from_path: from_path.into(),
            to_path: to_path.into(),
            change,
        }
    }

    /// The name git shows in the stat's left column: a rename or copy shows the
    /// compact `old => new` form ([`pprint_rename`]), every other status shows
    /// the path itself.
    fn print_name(&self) -> String {
        match self.status.kind() {
            ChangeKind::Renamed | ChangeKind::Copied => {
                pprint_rename(&self.from_path, &self.to_path)
            }
            _ => quote_path(&self.path()),
        }
    }

    /// The single path of a non-rename change: the new side for a create or
    /// modify, the old side for a delete (which has no new side).
    fn path(&self) -> String {
        if matches!(self.status.kind(), ChangeKind::Deleted) {
            self.from_path.clone()
        } else {
            self.to_path.clone()
        }
    }

    /// Whether the stat shows this entry. git drops only a boring file — a plain
    /// content modification with no line change and no mode change; a binary diff
    /// (always a real change), a create / delete / rename / copy / type change, or
    /// a pure mode change is always shown.
    fn is_shown(&self) -> bool {
        match self.change {
            StatChange::Binary { .. } => true,
            StatChange::Text { added, deleted } => {
                added + deleted > 0
                    || !matches!(self.status.kind(), ChangeKind::Modified)
                    || self.from_mode != self.to_mode
            }
        }
    }

    /// The added/deleted line totals for the summary (zero for a binary file,
    /// which git counts in neither the insertions nor deletions tally).
    fn line_totals(&self) -> (u64, u64) {
        match self.change {
            StatChange::Text { added, deleted } => (u64::from(added), u64::from(deleted)),
            StatChange::Binary { .. } => (0, 0),
        }
    }

    /// This entry's `--summary` line (create / delete / rename / copy / mode
    /// change), or empty when it has none. Mirrors git's `diff_summary`.
    fn summary_line(&self) -> String {
        match self.status.kind() {
            ChangeKind::Added => file_mode_name("create", self.to_mode, &self.to_path),
            ChangeKind::Deleted => file_mode_name("delete", self.from_mode, &self.from_path),
            ChangeKind::Renamed => self.rename_copy_line("rename"),
            ChangeKind::Copied => self.rename_copy_line("copy"),
            ChangeKind::Modified | ChangeKind::TypeChanged => self.mode_change_line(true),
        }
    }

    /// git's `show_rename_copy`: the `rename`/`copy old => new (NN%)` line,
    /// followed by a nameless mode-change line when the modes also differ.
    fn rename_copy_line(&self, verb: &str) -> String {
        let names: String = pprint_rename(&self.from_path, &self.to_path);
        let mut line: String = format!(" {verb} {names} ({}%)\n", self.status.similarity());
        line.push_str(&self.mode_change_line(false));
        line
    }

    /// git's `show_mode_change`: ` mode change <old> => <new>` with the path
    /// appended only when `show_name` is set (a plain mode change names the file;
    /// a rename/copy already named it). Empty when both modes are present and
    /// equal, or either is unset.
    fn mode_change_line(&self, show_name: bool) -> String {
        if !self.from_mode.is_set() || !self.to_mode.is_set() || self.from_mode == self.to_mode {
            return String::new();
        }
        let mut line: String = format!(
            " mode change {} => {}",
            self.from_mode.octal(),
            self.to_mode.octal()
        );
        if show_name {
            line.push(' ');
            line.push_str(&quote_path(&self.to_path));
        }
        line.push('\n');
        line
    }
}

/// A whole diffstat: the file entries of one diff, in git's display order (the
/// order the patch presents the files).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diffstat {
    entries: Vec<DiffstatEntry>,
}

impl Diffstat {
    /// Builds a diffstat from its entries.
    #[must_use]
    pub fn new(entries: Vec<DiffstatEntry>) -> Self {
        Self { entries }
    }

    /// Whether the diff touched no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Renders the full diffstat block: the `--stat` lines, the summary line,
    /// then the `--summary` lines — every line newline terminated, exactly as
    /// `git diff --stat --summary` writes it.
    #[must_use]
    pub fn render(&self) -> String {
        // git drops a boring zero-change modification; everything else is shown.
        let shown: Vec<&DiffstatEntry> = self
            .entries
            .iter()
            .filter(|entry: &&DiffstatEntry| entry.is_shown())
            .collect();

        let mut out: String = String::new();
        let layout: Layout = Layout::measure(&shown);
        for entry in &shown {
            layout.write_stat_line(entry, &mut out);
        }
        out.push_str(&self.summary_totals(shown.len()));
        for entry in &self.entries {
            out.push_str(&entry.summary_line());
        }
        out
    }

    /// The ` N files changed, …` line. Mirrors git's
    /// `print_stat_summary_inserts_deletes`.
    fn summary_totals(&self, shown_files: usize) -> String {
        let (mut insertions, mut deletions): (u64, u64) = (0, 0);
        for entry in &self.entries {
            let (added, deleted): (u64, u64) = entry.line_totals();
            insertions += added;
            deletions += deleted;
        }
        if shown_files == 0 {
            return " 0 files changed\n".to_owned();
        }

        let mut sb: String = format!(
            " {shown_files} file{} changed",
            if shown_files == 1 { "" } else { "s" }
        );
        // git prints the insertions clause unless there are deletions only.
        if insertions > 0 || deletions == 0 {
            sb.push_str(&format!(
                ", {insertions} insertion{}(+)",
                if insertions == 1 { "" } else { "s" }
            ));
        }
        // …and the deletions clause unless there are insertions only.
        if deletions > 0 || insertions == 0 {
            sb.push_str(&format!(
                ", {deletions} deletion{}(-)",
                if deletions == 1 { "" } else { "s" }
            ));
        }
        sb.push('\n');
        sb
    }
}

/// The measured column widths of a diffstat, computed once over every shown
/// file the way git's `show_stats` does, then applied per line.
struct Layout {
    name_width: i64,
    number_width: i64,
    graph_width: i64,
    max_change: u64,
}

impl Layout {
    /// Measures the columns from the shown entries, faithful to `show_stats`'s
    /// width arithmetic (with git's unset `stat_name_width` / `stat_graph_width`,
    /// so the name and graph are bounded only by the total width).
    fn measure(shown: &[&DiffstatEntry]) -> Self {
        let mut max_len: i64 = 0;
        let mut max_change: u64 = 0;
        let mut number_width: i64 = 0;
        let mut bin_width: i64 = 0;
        for entry in shown {
            let len: i64 = display_width(&entry.print_name());
            if max_len < len {
                max_len = len;
            }
            match entry.change {
                StatChange::Binary { old_size, new_size } => {
                    // "Bin XXX -> YYY bytes" — git reserves its width and pins
                    // the number column to 3 ("Bin").
                    let w: i64 = 14 + decimal_width(old_size) + decimal_width(new_size);
                    if bin_width < w {
                        bin_width = w;
                    }
                    number_width = 3;
                }
                StatChange::Text { added, deleted } => {
                    let change: u64 = u64::from(added) + u64::from(deleted);
                    if max_change < change {
                        max_change = change;
                    }
                }
            }
        }

        let mut width: i64 = STAT_WIDTH;
        let change_digits: i64 = decimal_width(max_change);
        if change_digits > number_width {
            number_width = change_digits;
        }
        // git guarantees room for a minimal name + graph.
        if width < 16 + 6 + number_width {
            width = 16 + 6 + number_width;
        }

        let mut graph_width: i64 = if max_change as i64 + 4 > bin_width {
            max_change as i64
        } else {
            bin_width - 4
        };
        let mut name_width: i64 = max_len;

        // Shrink the adjustable widths to fit, exactly as git does.
        if name_width + number_width + 6 + graph_width > width {
            if graph_width > width * 3 / 8 - number_width - 6 {
                graph_width = width * 3 / 8 - number_width - 6;
                if graph_width < 6 {
                    graph_width = 6;
                }
            }
            if name_width > width - number_width - 6 - graph_width {
                name_width = width - number_width - 6 - graph_width;
            } else {
                graph_width = width - number_width - 6 - name_width;
            }
        }

        Self {
            name_width,
            number_width,
            graph_width,
            max_change,
        }
    }

    /// Appends one file's `--stat` line.
    fn write_stat_line(&self, entry: &DiffstatEntry, out: &mut String) {
        let (prefix, name, name_field): (&str, String, i64) = self.fit_name(&entry.print_name());
        let padding: usize = (name_field - display_width(&name)).max(0) as usize;
        let number_width: usize = self.number_width.max(0) as usize;

        match entry.change {
            StatChange::Binary { old_size, new_size } => {
                out.push_str(&format!(
                    " {prefix}{name}{:padding$} | {:>number_width$} {old_size} -> {new_size} bytes\n",
                    "", "Bin",
                ));
            }
            StatChange::Text { added, deleted } => {
                let total: u64 = u64::from(added) + u64::from(deleted);
                let (adds, dels): (u64, u64) =
                    self.scale_graph(u64::from(added), u64::from(deleted));
                let trailing: &str = if total > 0 { " " } else { "" };
                out.push_str(&format!(
                    " {prefix}{name}{:padding$} | {total:>number_width$}{trailing}",
                    "",
                ));
                out.push_str(&"+".repeat(adds as usize));
                out.push_str(&"-".repeat(dels as usize));
                out.push('\n');
            }
        }
    }

    /// Scales an add/delete pair to the graph width, git's per-line `scale_linear`
    /// step. When the changes fit, the counts pass through unchanged; when they
    /// would overflow, they are scaled with at least one bar kept per side.
    fn scale_graph(&self, added: u64, deleted: u64) -> (u64, u64) {
        if self.graph_width as u64 > self.max_change {
            return (added, deleted);
        }
        let mut total: u64 = scale_linear(added + deleted, self.graph_width, self.max_change);
        if total < 2 && added > 0 && deleted > 0 {
            total = 2;
        }
        if added < deleted {
            let adds: u64 = scale_linear(added, self.graph_width, self.max_change);
            (adds, total - adds)
        } else {
            let dels: u64 = scale_linear(deleted, self.graph_width, self.max_change);
            (total - dels, dels)
        }
    }

    /// git's name "scaling": a name within the column passes through; a longer
    /// one is truncated from the left to a `...`-prefixed tail, snapped to a path
    /// separator when one survives. Returns the prefix, the (possibly truncated)
    /// name, and the field width to pad it within.
    fn fit_name(&self, name: &str) -> (&'static str, String, i64) {
        let name_len: i64 = display_width(name);
        if self.name_width >= name_len {
            return ("", name.to_owned(), self.name_width);
        }
        let field: i64 = (self.name_width - 3).max(0);
        let chars: Vec<char> = name.chars().collect();
        let mut start: usize = 0;
        let mut remaining: i64 = name_len;
        while remaining > field && start < chars.len() {
            remaining -= char_width(chars[start]);
            start += 1;
        }
        let tail: String = chars[start..].iter().collect();
        let snapped: String = match tail.find('/') {
            Some(idx) => tail[idx..].to_owned(),
            None => tail,
        };
        ("...", snapped, field)
    }
}

/// git's `scale_linear`: scale `it` onto `graph_width`, keeping at least one bar
/// for any non-zero change (it scales as if the width were one shorter, then
/// adds one). `max_change` is positive whenever `it` is, so the division is safe.
fn scale_linear(it: u64, graph_width: i64, max_change: u64) -> u64 {
    if it == 0 {
        return 0;
    }
    1 + (it * (graph_width as u64).saturating_sub(1) / max_change)
}

/// git's `decimal_width`: the number of digits, at least one (so `0` is width 1).
fn decimal_width(mut number: u64) -> i64 {
    let mut width: i64 = 1;
    while number >= 10 {
        width += 1;
        number /= 10;
    }
    width
}

/// git's `show_file_mode_name`: ` <verb> mode <octal> <path>` when the mode is
/// set, ` <verb> <path>` otherwise. The path is C-style quoted.
fn file_mode_name(verb: &str, mode: FileMode, path: &str) -> String {
    if mode.is_set() {
        format!(" {verb} mode {} {}\n", mode.octal(), quote_path(path))
    } else {
        format!(" {verb} {}\n", quote_path(path))
    }
}

/// The display width of a string. ASCII and BMP-narrow text is one column per
/// character — the corpus and git's own paths; full East-Asian-width handling is
/// a deeper edge git's `utf8_strwidth` covers that the paths here never reach.
fn display_width(text: &str) -> i64 {
    text.chars().map(char_width).sum()
}

/// The display width of a single character (one column for the text handled
/// here), the per-character half of [`display_width`].
fn char_width(_ch: char) -> i64 {
    1
}

/// git's `quote_c_style`: a path with no byte needing an escape is shown
/// verbatim; one that does is wrapped in double quotes with C escapes. The paths
/// the corpus and ordinary repositories carry take the verbatim branch.
fn quote_path(path: &str) -> String {
    if !needs_quoting(path) {
        return path.to_owned();
    }
    let mut out: String = String::from('"');
    for byte in path.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x07 => out.push_str("\\a"),
            0x08 => out.push_str("\\b"),
            0x0c => out.push_str("\\f"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x0b => out.push_str("\\v"),
            0x20..=0x7e => out.push(byte as char),
            other => out.push_str(&format!("\\{other:03o}")),
        }
    }
    out.push('"');
    out
}

/// Whether a path has any byte git's `quote_c_style` would escape: a control
/// byte, a high (non-ASCII) byte, a double quote, or a backslash.
fn needs_quoting(path: &str) -> bool {
    path.bytes()
        .any(|byte: u8| !(0x20..0x7f).contains(&byte) || byte == b'"' || byte == b'\\')
}

/// git's `pprint_rename`: the compact rename/copy display, `old => new` collapsed
/// to `pfx{mid-a => mid-b}sfx` around any shared leading directory and trailing
/// suffix. A name needing C-style quoting takes git's simple `"a" => "b"` form.
fn pprint_rename(old_name: &str, new_name: &str) -> String {
    if needs_quoting(old_name) || needs_quoting(new_name) {
        return format!("{} => {}", quote_path(old_name), quote_path(new_name));
    }

    let a: &[u8] = old_name.as_bytes();
    let b: &[u8] = new_name.as_bytes();

    // Common prefix, recorded up to and including the last shared '/'.
    let mut pfx_length: usize = 0;
    let mut i: usize = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        if a[i] == b'/' {
            pfx_length = i + 1;
        }
        i += 1;
    }

    // Common suffix, recorded up to and including the last shared '/'. The walk
    // may run one byte into the prefix to see that same slash.
    let pfx_adjust: usize = usize::from(pfx_length > 0);
    let mut sfx_length: usize = 0;
    let mut ia: usize = a.len();
    let mut ib: usize = b.len();
    while ia > pfx_length - pfx_adjust
        && ib > pfx_length - pfx_adjust
        && ia > 0
        && ib > 0
        && a[ia - 1] == b[ib - 1]
    {
        if a[ia - 1] == b'/' {
            sfx_length = a.len() - (ia - 1);
        }
        ia -= 1;
        ib -= 1;
    }

    let pfx: &str = &old_name[..pfx_length];
    let sfx: &str = &old_name[a.len() - sfx_length..];
    let mid_a: &str = &old_name[pfx_length..a.len() - sfx_length];
    let mid_b: &str = &new_name[pfx_length..b.len() - sfx_length];

    if pfx_length > 0 || sfx_length > 0 {
        format!("{pfx}{{{mid_a} => {mid_b}}}{sfx}")
    } else {
        format!("{old_name} => {new_name}")
    }
}
