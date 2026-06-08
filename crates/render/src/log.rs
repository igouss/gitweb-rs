//! The verbose log (modernized `git_log` / `git_log_body`).
//!
//! Where the shortlog is one table row per commit, the log is a block per commit:
//! a header carrying the commit's relative age and subject (linking to the
//! commit), the per-commit `commit | commitdiff | tree` links, an authorship line
//! with the author and the absolute date the commit was authored, and the message
//! body — one entry per processed line, with sign-off and link trailers styled
//! apart (the domain's [`LogLine`] classification, gitweb's `git_print_log`).
//!
//! The date swap, the age, and the line classification are the domain's calls;
//! the timestamp markup is the shared [`timestamp_badge`]. URLs are gitweb's
//! `href()`, the web boundary's job, so every link arrives finished. The ref
//! markers, avatars, and the SHA-token links gitweb also weaves in are their own
//! features and are not wired here.

use gitweb_domain::model::message_body::LogLine;
use gitweb_domain::model::timestamp::Timestamp;

use crate::chrome::{Crumb, MoreLink, NavItem, PageFooter, footer, page_header, page_nav};
use crate::markup::{Markup, html};
use crate::timestamp::timestamp_badge;

/// One verbose log entry: the header age and subject, the authorship, the message
/// body, and the per-commit links the boundary built.
#[derive(Debug, Clone)]
pub struct LogEntryView {
    /// The relative age shown in the header (gitweb's `age_string`).
    pub age: String,
    /// The commit subject shown in the header; links to the commit.
    pub title: String,
    /// The author's display name.
    pub author: String,
    /// The author's absolute date, rendered as the shared timestamp badge.
    pub timestamp: Timestamp,
    /// The processed message body lines (gitweb's `git_print_log`).
    pub comment: Vec<LogLine>,
    /// `commit` URL for this commit; the header subject links here too.
    pub commit: String,
    /// `commitdiff` URL for this commit.
    pub commitdiff: String,
    /// `tree` URL for this commit.
    pub tree: String,
}

/// The whole verbose-log page body: the breadcrumb header, the action navigation,
/// the commit blocks, and the optional trailing "more" link.
#[derive(Debug, Clone)]
pub struct LogPage {
    /// Breadcrumb trail (home / project / log).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The commit blocks, already ordered by the use case.
    pub entries: Vec<LogEntryView>,
    /// The "more" affordance, present only when a further page exists.
    pub more: Option<MoreLink>,
}

/// Renders the verbose-log page body: the breadcrumb header, the action
/// navigation, the commit blocks inside a `main` element (or a note when there
/// are no commits), and the footer. The caller wraps this in the document
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn log_body(page: &LogPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { "Log" }
            @if page.entries.is_empty() {
                p class="note" { "No commits." }
            } @else {
                (log_entries(&page.entries, page.more.as_ref()))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the commit blocks, then the optional trailing "more" link.
#[must_use]
pub fn log_entries(entries: &[LogEntryView], more: Option<&MoreLink>) -> Markup {
    html! {
        @for entry in entries {
            (log_entry(entry))
        }
        @if let Some(more) = more {
            nav class="page-nav log-more" {
                a href=(more.href) { (more.label) }
            }
        }
    }
}

/// One commit block: the header (age + subject linking to the commit), the
/// per-commit links and authorship, and the message body.
fn log_entry(entry: &LogEntryView) -> Markup {
    html! {
        article class="log-entry" {
            div class="header" {
                a class="title" href=(entry.commit) {
                    span class="age" { (entry.age) }
                    " "
                    (entry.title)
                }
            }
            div class="title_text" {
                div class="log_link" {
                    a href=(entry.commit) { "commit" }
                    " | "
                    a href=(entry.commitdiff) { "commitdiff" }
                    " | "
                    a href=(entry.tree) { "tree" }
                }
                span class="author_date" {
                    (entry.author) " [" (timestamp_badge(&entry.timestamp)) "]"
                }
            }
            div class="log_body" {
                @for line in &entry.comment {
                    (log_line(line))
                }
            }
        }
    }
}

/// One processed message line: prose (escaped, then a break), a sign-off in its
/// own span, or a link trailer with its URL linked — each followed by a `<br>`,
/// mirroring gitweb's `git_print_log`.
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
