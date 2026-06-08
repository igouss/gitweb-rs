//! A commit message rendered as a log body — gitweb's `git_print_log`.
//!
//! `git_log_body` shows each commit's full message through `git_print_log`, which
//! is a stateful line filter, not a verbatim dump:
//!
//!   * leading blank lines are dropped;
//!   * a run of blank lines collapses to a single one;
//!   * a sign-off / trailer line (`Signed-off-by:`, `Acked-by:`, `Cc:`,
//!     `Closes:`, `Fixes:`, …) is classified apart so the view can style it, and
//!     it suppresses the blank line that follows it;
//!   * an `xxxlink: <url>` trailer is recognised so its URL becomes a link;
//!   * the block ends with one trailing blank line, unless the last emitted line
//!     was already blank or a trailer.
//!
//! This module owns those pure rules and returns the classified lines; the render
//! layer turns each [`LogLine`] into markup. gitweb also runs each prose line
//! through `format_log_line_html`, which turns abbreviated object ids into links
//! to the `object` view — that needs the boundary's `href()`, so it is left to a
//! later refinement, exactly as the shortlog left its ref markers and avatars.
//!
//! There is no regex crate in the domain (it depends on nothing), so the trailer
//! shapes gitweb matches with regular expressions are recognised here by hand,
//! the same way [`crate::model::signature`] hand-parses an ident line.

/// One processed line of a commit's message body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLine {
    /// A prose line. An empty string is a blank spacer line (a `<br/>`).
    Text(String),
    /// A sign-off / trailer line, styled apart from prose (gitweb's `signoff`
    /// span). Carries the whole line verbatim.
    Signoff(String),
    /// An `xxxlink: <url>` trailer whose URL gitweb renders as a link.
    Autolink {
        /// The trailer label (the part before the colon, e.g. `Link`).
        label: String,
        /// The linked URL (an `http`/`https` address).
        url: String,
    },
}

/// Splits a commit message into its display log body, applying gitweb's
/// `git_print_log` rules (with its `-final_empty_line` option, the only mode
/// `git_log_body` uses).
///
/// The message is taken as gitweb's `comment` array: its lines in order,
/// including the subject as the first line (gitweb shows the subject in the
/// header *and* repeats it here, since it does not pass `-remove_title`).
#[must_use]
pub fn log_lines(message: &str) -> Vec<LogLine> {
    let mut out: Vec<LogLine> = Vec::new();
    // gitweb's `$skip_blank_line`: set after a blank or a trailer, so the next
    // blank line is swallowed. It starts false; the message's leading blank
    // lines are dropped first, as gitweb does in a separate pass.
    let mut skip_blank: bool = false;
    for line in message.lines().skip_while(|line: &&str| line.is_empty()) {
        if let Some(trailer) = classify_trailer(line) {
            out.push(trailer);
            skip_blank = true;
            continue;
        }
        if line.is_empty() {
            if skip_blank {
                continue;
            }
            skip_blank = true;
        } else {
            skip_blank = false;
        }
        out.push(LogLine::Text(line.to_owned()));
    }
    // `-final_empty_line`: end on a single blank unless we already are blank.
    if !skip_blank {
        out.push(LogLine::Text(String::new()));
    }
    out
}

/// Classifies a line as a sign-off or an autolink trailer, or `None` for prose.
/// gitweb checks the sign-off shape first, so a line that fits both is a
/// sign-off.
fn classify_trailer(line: &str) -> Option<LogLine> {
    if is_signoff(line) {
        return Some(LogLine::Signoff(line.to_owned()));
    }
    parse_autolink(line)
}

/// Whether `line` is a sign-off / trailer, mirroring gitweb's
/// `^\s*([A-Z][-A-Za-z]*-([Bb]y|[Tt]o)|C[Cc]|(Clos|Fix)es): `.
fn is_signoff(line: &str) -> bool {
    let trimmed: &str = line.trim_start();
    let Some(colon) = trimmed.find(": ") else {
        return false;
    };
    signoff_key(&trimmed[..colon])
}

/// Whether `key` (the text before the `": "`) is one of gitweb's trailer keys:
/// `Cc`, `Closes`, `Fixes`, or an `[A-Z][-A-Za-z]*-(by|to)` form (case as in the
/// regex: the final word is `By`/`by`/`To`/`to`).
fn signoff_key(key: &str) -> bool {
    matches!(key, "CC" | "Cc" | "Closes" | "Fixes") || ends_in_by_or_to(key)
}

/// Whether `key` matches `[A-Z][-A-Za-z]*-([Bb]y|[Tt]o)`: an uppercase-led run
/// of letters and hyphens ending in `-by`/`-By`/`-to`/`-To`.
fn ends_in_by_or_to(key: &str) -> bool {
    let suffixes: [&str; 4] = ["-by", "-By", "-to", "-To"];
    let Some(head) = suffixes
        .iter()
        .find_map(|suffix: &&str| key.strip_suffix(suffix))
    else {
        return false;
    };
    let mut chars = head.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && chars.all(|c: char| c == '-' || c.is_ascii_alphabetic())
}

/// Recognises an `xxxlink: <url>` trailer, mirroring gitweb's
/// `\s*([a-z]*link): (https?://\S+)` (case-insensitively). gitweb's pattern is
/// unanchored; this anchors it to the line start (after whitespace), since a
/// mid-line match would silently drop the surrounding prose.
fn parse_autolink(line: &str) -> Option<LogLine> {
    let trimmed: &str = line.trim_start();
    let colon: usize = trimmed.find(": ")?;
    let label: &str = &trimmed[..colon];
    if !is_link_label(label) {
        return None;
    }
    let rest: &str = trimmed[colon + 2..].trim_start();
    let url: &str = rest.split_whitespace().next()?;
    if !is_http_url(url) {
        return None;
    }
    Some(LogLine::Autolink {
        label: label.to_owned(),
        url: url.to_owned(),
    })
}

/// Whether `label` is `[A-Za-z]*link` (case-insensitive): a run of ASCII letters
/// ending in `link`.
fn is_link_label(label: &str) -> bool {
    label.chars().all(|c: char| c.is_ascii_alphabetic())
        && label.to_ascii_lowercase().ends_with("link")
}

/// Whether `url` is an `http`/`https` address with at least one character after
/// the scheme, mirroring gitweb's `https?://\S+`.
fn is_http_url(url: &str) -> bool {
    let lower: String = url.to_ascii_lowercase();
    for scheme in ["http://", "https://"] {
        if let Some(rest) = lower.strip_prefix(scheme) {
            return !rest.is_empty();
        }
    }
    false
}
