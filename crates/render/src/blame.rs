//! Rendering blame: the server-rendered table, the incremental shell, and the
//! blame_data stream (modernized `git_blame_common`).
//!
//! gitweb renders blame three ways from one file. The *server-rendered* table
//! (porcelain mode) shows one `<tr id="lN">` per final line: the first line of a
//! commit group carries a `sha1` cell (spanning the group via `rowspan`, the short
//! id linking to the commit, the author and date in its `title`), every line
//! carries a `linenr` cell linking to that line's source and the line's text. The
//! *incremental shell* serves the same rows empty — a `sha1` cell with a bare
//! anchor and a `linenr` cell with the line number — plus a `progress_bar` /
//! `progress_info` and an inline module that hands the blame_data URL and the
//! project URL to the shipped `blame-incremental.js` (`startBlame`), which fills
//! the rows as the data streams in. The *blame_data* stream is the plain `git
//! blame --incremental` text that module parses.
//!
//! Every URL is built by the web boundary, so this layer takes finished hrefs and
//! only places them; every value is escaped by maud. The `class` names the shipped
//! client module reads (`sha1`, `linenr`, `color`/`light`/`dark`) and the element
//! ids (`l<N>`, `blame_table`, `progress_bar`, `progress_info`) are part of that
//! contract and are emitted verbatim.

use std::fmt::Write as _;

use crate::chrome::{Crumb, FormatLink, NavItem, formats_nav, page_header, page_nav};
use crate::markup::{Markup, PreEscaped, html};

/// One line of a server-rendered blame group after the first: its final line
/// number (the `id="lN"` row), the `linenr` cell's link and the original line
/// number it shows, and the line's text.
#[derive(Debug, Clone)]
pub struct BlameTableLine {
    /// The final line number — the `id="lN"` row and the number the `linenr` cell
    /// shows.
    pub final_lineno: usize,
    /// The original line number, the `#lN` fragment the `linenr` link targets.
    pub orig_lineno: usize,
    /// The finished `linenr` link href (the line's source, gitweb's `$blamed#lN`).
    pub linenr_href: String,
    /// The line's text.
    pub text: String,
}

/// One commit group of the server-rendered table: the lines it spans, the short id
/// and the commit link the `sha1` cell shows, and the author/date tooltip.
#[derive(Debug, Clone)]
pub struct BlameTableGroup {
    /// The 8-char short id shown in the `sha1` cell.
    pub short_commit: String,
    /// The finished commit link href.
    pub commit_href: String,
    /// The `sha1` cell tooltip: `"<author>, <iso-tz date>"`.
    pub tooltip: String,
    /// The group's lines, in final-file order (at least one).
    pub lines: Vec<BlameTableLine>,
}

/// One line of the incremental shell: its final line number (the `id="lN"` row)
/// and its text. The `sha1`/`linenr` cells are served empty for the client to
/// fill.
#[derive(Debug, Clone)]
pub struct BlameShellLine {
    /// The final line number — the `id="lN"` row the client fills.
    pub final_lineno: usize,
    /// The line's text.
    pub text: String,
}

/// One record of the blame_data stream: the commit, the line numbers and span of
/// the group's header, and the commit's author identity and filename.
#[derive(Debug, Clone)]
pub struct BlameDataGroup {
    /// The full commit id.
    pub commit: String,
    /// The original line number of the group's first line.
    pub orig_lineno: usize,
    /// The final line number of the group's first line.
    pub final_lineno: usize,
    /// The number of lines in the group.
    pub numlines: usize,
    /// The commit author's display name.
    pub author: String,
    /// The author time as a Unix epoch.
    pub author_time: i64,
    /// The author's timezone token (e.g. `"+0900"`).
    pub author_tz: String,
    /// The file's name in this commit (the `filename` line ending the record).
    pub filename: String,
}

/// The server-rendered blame page body: the breadcrumb header, the action
/// navigation with the file affordances, the commit subject, the path, and the
/// filled table.
#[derive(Debug, Clone)]
pub struct BlamePage {
    /// Breadcrumb trail (home / project / blame).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation.
    pub nav: Vec<NavItem>,
    /// The file affordances (blob / blame variant / history / HEAD).
    pub formats: Vec<FormatLink>,
    /// The header title: the base commit's subject.
    pub title: String,
    /// The tracked file path.
    pub path: String,
    /// The blame groups, in final-file order.
    pub groups: Vec<BlameTableGroup>,
}

/// The incremental blame page body: the same chrome as [`BlamePage`], plus the
/// empty-row shell, the progress elements, and the bootstrap of the client module.
#[derive(Debug, Clone)]
pub struct BlameIncrementalPage {
    /// Breadcrumb trail (home / project / blame).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation.
    pub nav: Vec<NavItem>,
    /// The file affordances (blob / blame variant / history / HEAD).
    pub formats: Vec<FormatLink>,
    /// The header title: the base commit's subject.
    pub title: String,
    /// The tracked file path.
    pub path: String,
    /// The file's lines, served as empty rows for the client to fill.
    pub lines: Vec<BlameShellLine>,
    /// The blame_data URL the client streams (`startBlame`'s first argument).
    pub data_url: String,
    /// The project's base URL the client builds links from (`startBlame`'s second).
    pub project_url: String,
    /// The `src` of the client module that fills the rows (a static asset).
    pub module_src: String,
}

/// Renders the server-rendered blame page body. The caller wraps it in the
/// document chrome.
#[must_use]
pub fn blame_body(page: &BlamePage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, formats_nav(&page.formats)))
        main class="content" {
            h2 class="section-title" { (page.title) }
            p class="path" { (page.path) }
            (blame_table(&page.groups))
        }
    }
}

/// Renders the incremental blame page body: the chrome, the progress elements,
/// the empty-row shell, and the inline bootstrap module. The caller wraps it in
/// the document chrome.
#[must_use]
pub fn blame_incremental_body(page: &BlameIncrementalPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, formats_nav(&page.formats)))
        main class="content" {
            h2 class="section-title" { (page.title) }
            p class="path" { (page.path) }
            noscript {
                p class="error" { "This page requires JavaScript to fill the blame." }
            }
            div id="progress_bar" class="progress-bar" {}
            div id="progress_info" { "… / …" }
            (incremental_shell(&page.lines))
        }
        (bootstrap(&page.module_src, &page.data_url, &page.project_url))
    }
}

/// Renders the filled server-rendered table: one `<tr id="lN">` per line, the
/// first line of each group carrying the spanning `sha1` cell.
#[must_use]
pub fn blame_table(groups: &[BlameTableGroup]) -> Markup {
    html! {
        table id="blame_table" class="blame" {
            thead { tr { th { "Commit" } th { "Line" } th { "Data" } } }
            tbody {
                @for group in groups {
                    (table_group(group))
                }
            }
        }
    }
}

/// Renders one group: the first line carries the spanning `sha1` cell; every line
/// carries its `linenr` cell and text.
fn table_group(group: &BlameTableGroup) -> Markup {
    html! {
        @for (index, line) in group.lines.iter().enumerate() {
            tr id=(format!("l{}", line.final_lineno)) {
                @if index == 0 {
                    td class="sha1" rowspan=(group.lines.len()) title=(group.tooltip) {
                        a href=(group.commit_href) { (group.short_commit) }
                    }
                }
                td class="linenr" {
                    a class="linenr" href=(line.linenr_href) { (line.final_lineno) }
                }
                td class="pre" { (line.text) }
            }
        }
    }
}

/// Renders the empty-row shell the client fills: one `<tr id="lN">` per line with
/// an empty `sha1` anchor, an empty `linenr` anchor showing the number, and the
/// line text.
fn incremental_shell(lines: &[BlameShellLine]) -> Markup {
    html! {
        table id="blame_table" class="blame" {
            thead { tr { th { "Commit" } th { "Line" } th { "Data" } } }
            tbody {
                @for line in lines {
                    tr id=(format!("l{}", line.final_lineno)) class="light" {
                        td class="sha1" { a href="" { " " } }
                        td class="linenr" { a class="linenr" href="" { (line.final_lineno) } }
                        td class="pre" { (line.text) }
                    }
                }
            }
        }
    }
}

/// Renders the inline bootstrap module: imports `startBlame` from the client
/// module and calls it with the blame_data URL and the project URL (gitweb's
/// `startBlame("<blame_data>", "<base>")`). The URLs are JSON-escaped into JS
/// string literals so a `"` or `\` in a href cannot break out of the call.
fn bootstrap(module_src: &str, data_url: &str, project_url: &str) -> Markup {
    let script: String = format!(
        "import {{ startBlame }} from {};\nstartBlame({}, {});\n",
        js_string(module_src),
        js_string(data_url),
        js_string(project_url),
    );
    html! {
        script type="module" { (PreEscaped(script)) }
    }
}

/// Renders the blame_data stream: one `git blame --incremental` record per group
/// (`<sha> <orig> <final> <numlines>`, then `author` / `author-time` /
/// `author-tz` / `filename`), terminated by gitweb's `END` footer line.
#[must_use]
pub fn blame_data(groups: &[BlameDataGroup]) -> String {
    let mut out: String = String::new();
    for group in groups {
        data_record(&mut out, group);
    }
    out.push_str("END\n");
    out
}

/// Appends one group's `git blame --incremental` record to `out`. `filename` is
/// the last line of a record — it is what tells the client the record is complete
/// (its parser fills the group on the `filename` line).
fn data_record(out: &mut String, group: &BlameDataGroup) {
    let _ = writeln!(
        out,
        "{} {} {} {}",
        group.commit, group.orig_lineno, group.final_lineno, group.numlines
    );
    let _ = writeln!(out, "author {}", group.author);
    let _ = writeln!(out, "author-time {}", group.author_time);
    let _ = writeln!(out, "author-tz {}", group.author_tz);
    let _ = writeln!(out, "filename {}", group.filename);
}

/// Encodes `value` as a JS/JSON string literal (quoted, with `"`, `\`, and
/// control characters escaped) — safe to drop into the inline `<script
/// type="module">` bootstrap.
fn js_string(value: &str) -> String {
    let mut out: String = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
