//! A configured git remote and the URL lines its remotes-view block shows.
//!
//! Mirrors gitweb's `git_get_remotes_list` (a remote's name and the fetch/push
//! URLs `git remote -v` reports) and the URL-table rule inside `git_remote_block`:
//! given which of the fetch and push URLs are set, decide which rows to show.
//! When both are set and equal it is one combined "URL" row; when they differ it
//! is a "Fetch URL" row and a "Push URL" row; with only one it is that one row;
//! with neither it is a single "No remote URL" placeholder. The display labels
//! belong to the render layer — this entity owns the *rule*, not the text.
//!
//! The remote-tracking branches under `refs/remotes/<name>/` are not part of this
//! entity: the remotes use case reads them through the [`Repository`] port and
//! enriches them with the same rule the heads listing uses.
//!
//! [`Repository`]: crate::port::repository::Repository

/// One row of a remote's URL block, the semantic shape gitweb's `git_remote_block`
/// chooses before `format_repo_url` labels it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteUrl {
    /// Fetch and push are the same URL — gitweb's single "URL" row.
    Combined(String),
    /// The fetch URL, shown distinctly — gitweb's "Fetch URL" row.
    Fetch(String),
    /// The push URL, shown distinctly — gitweb's "Push URL" row.
    Push(String),
    /// No URL is configured — gitweb's empty-label "No remote URL" placeholder.
    Missing,
}

/// A configured git remote: its name and the fetch/push URLs `git remote -v`
/// reports. Either URL may be absent (gitweb leaves a remote out of its hash when
/// `git remote -v` emits no line for it, but the URL-line rule still handles the
/// missing cases the way `git_remote_block` does).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    name: String,
    fetch_url: Option<String>,
    push_url: Option<String>,
}

impl Remote {
    /// A remote from its name and optional fetch and push URLs.
    #[must_use]
    pub fn new(name: String, fetch_url: Option<String>, push_url: Option<String>) -> Self {
        Self {
            name,
            fetch_url,
            push_url,
        }
    }

    /// The remote's name (`origin`, `upstream`, …).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The fetch URL, if one is configured.
    #[must_use]
    pub fn fetch_url(&self) -> Option<&str> {
        self.fetch_url.as_deref()
    }

    /// The push URL, if one is configured.
    #[must_use]
    pub fn push_url(&self) -> Option<&str> {
        self.push_url.as_deref()
    }

    /// The URL lines this remote's block shows, gitweb's `git_remote_block` rule:
    /// fetch and push equal collapse to one combined line; distinct fetch and push
    /// each get their own; a single set URL shows just that line; neither shows the
    /// placeholder.
    #[must_use]
    pub fn url_lines(&self) -> Vec<RemoteUrl> {
        match (self.fetch_url.as_deref(), self.push_url.as_deref()) {
            (Some(fetch), Some(push)) if fetch == push => {
                vec![RemoteUrl::Combined(fetch.to_owned())]
            }
            (Some(fetch), Some(push)) => {
                vec![
                    RemoteUrl::Fetch(fetch.to_owned()),
                    RemoteUrl::Push(push.to_owned()),
                ]
            }
            (Some(fetch), None) => vec![RemoteUrl::Fetch(fetch.to_owned())],
            (None, Some(push)) => vec![RemoteUrl::Push(push.to_owned())],
            (None, None) => vec![RemoteUrl::Missing],
        }
    }
}
