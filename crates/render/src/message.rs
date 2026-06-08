//! Shared commit-message body rendering — gitweb's `git_print_log` output.
//!
//! Both the verbose log and the commit view print a commit's processed message
//! body: prose lines, sign-off / trailer lines styled apart, and `xxxlink:`
//! trailers whose URL is linked. The classification is the domain's
//! ([`LogLine`], gitweb's `git_print_log`); this places it as markup, once, for
//! every view that shows a message body.

use gitweb_domain::model::message_body::LogLine;

use crate::markup::{Markup, html};

/// Renders the processed message lines: each prose line escaped and followed by a
/// break, each sign-off in its own span, each `xxxlink:` trailer with its URL
/// linked — mirroring gitweb's `git_print_log`.
#[must_use]
pub fn log_lines(lines: &[LogLine]) -> Markup {
    html! {
        @for line in lines {
            (log_line(line))
        }
    }
}

/// One processed message line: prose (escaped, then a break), a sign-off in its
/// own span, or a link trailer with its URL linked — each followed by a `<br>`.
fn log_line(line: &LogLine) -> Markup {
    html! {
        @match line {
            LogLine::Text(text) => { (text) br; }
            LogLine::Signoff(text) => { span class="signoff" { (text) } br; }
            LogLine::Autolink { label, url } => {
                span class="signoff" { (label) ": " a href=(url) { (url) } }
                br;
            }
        }
    }
}
