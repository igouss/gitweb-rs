//! Feed auto-discovery classification — gitweb's `get_feed_info`.
//!
//! Every gitweb HTML page advertises its RSS/Atom feeds in the document
//! `<head>` via `<link rel="alternate">` (gitweb's `print_feed_meta`). What
//! those links say — the feed's descriptive title and which ref/file it scopes
//! to — is decided per page by `get_feed_info`, the pure rule this module owns:
//!
//!   * actions `tags`, `heads`, `forks`, `tag`, `search`, and the actionless
//!     page get the generic project log (`-title => 'log'`, no scope);
//!   * otherwise, the page's `hash_base`/`hash` is matched against the
//!     configured branch directories (`heads` plus `extra-branch-refs`, gitweb's
//!     `get_branch_refs`); a match makes it a branch log and carries the
//!     fully-qualified `refs/<dir>/<branch>` into the link's `h` parameter;
//!   * a named file makes it a history (`tree` adds a trailing slash), combined
//!     with the branch when both are present.
//!
//! Wrapping this into the four RSS/Atom `<link>` tags (titles, MIME types) is
//! the render layer's job; building their URLs is the web boundary's. Here we
//! only classify.

/// The classified feed-discovery facts for one page: the descriptive title
/// gitweb's `print_feed_meta` wraps into each `<link>` title, the
/// fully-qualified branch ref the link's `h` parameter carries (when the page
/// scopes to a branch), and the file the feed narrows to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedInfo {
    title: String,
    hash: Option<String>,
    file_name: Option<String>,
}

impl FeedInfo {
    /// gitweb's `$href_params{-title}` — the descriptive feed type word(s)
    /// (`log`, `log of <branch>`, `history of <file>[ on '<branch>']`).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The fully-qualified `refs/<dir>/<branch>` the link's `h` parameter
    /// carries when the page scopes to a branch, or `None` otherwise.
    #[must_use]
    pub fn hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    /// The file the feed narrows to (the link's `f` parameter), or `None`.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }
}

/// gitweb's `get_feed_info`: classify a page into the feed-discovery facts its
/// `<head>` advertises.
///
/// `branch_refs` is gitweb's `get_branch_refs` — the branch directories under
/// `refs/` (`heads`, then the validated `extra-branch-refs`). `hash_base` and
/// `hash` are the page's `hb`/`h`; `file_name` is its `f`.
#[must_use]
pub fn feed_info(
    action: &str,
    hash_base: Option<&str>,
    hash: Option<&str>,
    file_name: Option<&str>,
    branch_refs: &[&str],
) -> FeedInfo {
    // gitweb returns early for these views (and the actionless page), so the
    // link falls back to the generic project log with no ref or file scope.
    if is_generic_feed_action(action) {
        return FeedInfo {
            title: "log".to_owned(),
            hash: None,
            file_name: None,
        };
    }
    let (hash, branch): (Option<String>, Option<&str>) =
        match match_branch(hash_base, hash, branch_refs) {
            Some((full_ref, branch)) => (Some(full_ref), Some(branch)),
            None => (None, None),
        };
    FeedInfo {
        title: title_for(action, file_name, branch),
        hash,
        file_name: file_name.map(str::to_owned),
    }
}

/// gitweb's `$action =~ /^(?:tags|heads|forks|tag|search)$/x` (plus the
/// actionless page): views that advertise only the generic project feed.
fn is_generic_feed_action(action: &str) -> bool {
    action.is_empty() || matches!(action, "tags" | "heads" | "forks" | "tag" | "search")
}

/// Matches the page's `hash_base` (then `hash`) against each configured branch
/// directory in order — gitweb's `$hash_base =~ m!^refs/\Q$ref\E/(.*)$!`. The
/// first directory that prefixes either ref wins, yielding the fully-qualified
/// `refs/<dir>/<branch>` (the link's `h` parameter) and the captured branch
/// name (gitweb's `$1`, everything after `refs/<dir>/` — so it is correct even
/// when the directory itself contains slashes).
fn match_branch<'a>(
    hash_base: Option<&'a str>,
    hash: Option<&'a str>,
    branch_refs: &[&str],
) -> Option<(String, &'a str)> {
    for &dir in branch_refs {
        let prefix: String = format!("refs/{dir}/");
        let branch: Option<&str> = hash_base
            .and_then(|reference: &str| reference.strip_prefix(prefix.as_str()))
            .or_else(|| hash.and_then(|reference: &str| reference.strip_prefix(prefix.as_str())));
        if let Some(branch) = branch {
            return Some((format!("{prefix}{branch}"), branch));
        }
    }
    None
}

/// gitweb's feed-type title (`-title`): a named file makes it a history (the
/// `tree` action adds a trailing slash, gitweb's `$type .= "/"`), combined with
/// the branch when both are present; otherwise a plain (or branch) log.
fn title_for(action: &str, file_name: Option<&str>, branch: Option<&str>) -> String {
    match file_name {
        Some(file) => {
            let slash: &str = if action == "tree" { "/" } else { "" };
            match branch {
                Some(name) => format!("history of {file}{slash} on '{name}'"),
                None => format!("history of {file}{slash}"),
            }
        }
        None => match branch {
            Some(name) => format!("log of {name}"),
            None => "log".to_owned(),
        },
    }
}
