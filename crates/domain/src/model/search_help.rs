//! The search-help topic list (gitweb's `git_search_help` `<dl>`).
//!
//! gitweb's search-help page documents each search type. Three of them —
//! `commit`, `author`, `committer` — are always documented: they search commit
//! metadata and need no extra feature beyond `search` itself. The other two are
//! gated, because each documents a search the user can only run when its feature
//! is on: `grep` (file-content search over a tree) when the `grep` feature is
//! enabled, and `pickaxe` (commits that change a string's occurrence count) when
//! the `pickaxe` feature is enabled.
//!
//! The order gitweb prints them is fixed — `commit`, then `grep`, then `author`,
//! `committer`, and finally `pickaxe` — so the gate only ever inserts a topic at
//! its fixed slot, never reorders the list. This is the page's only real rule;
//! the help prose and its markup are a view concern, so the rule yields the
//! ordered list of topics and the [`SearchHelpTopic::name`] each is keyed by
//! (which is also the `st` request value), and nothing else.

/// One search type documented on the search-help page, in the order gitweb
/// lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchHelpTopic {
    /// Search commit messages (`commit`) — always documented.
    Commit,
    /// Search file content over a tree (`grep`) — only when the grep feature is
    /// enabled.
    Grep,
    /// Search the change author's identity (`author`) — always documented.
    Author,
    /// Search the committer's identity (`committer`) — always documented.
    Committer,
    /// Search commits that change a string's occurrence count (`pickaxe`) — only
    /// when the pickaxe feature is enabled.
    Pickaxe,
}

impl SearchHelpTopic {
    /// The search type's name, as gitweb labels its `<dt>` and as the `st`
    /// request value that selects it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Grep => "grep",
            Self::Author => "author",
            Self::Committer => "committer",
            Self::Pickaxe => "pickaxe",
        }
    }
}

/// The search-help topics for the given feature configuration, in gitweb's fixed
/// order: `commit`, then `grep` when `grep_enabled`, then `author`, `committer`,
/// and finally `pickaxe` when `pickaxe_enabled`. The three always-present types
/// document searches that need no extra feature; the two gated types are only
/// documented when the user could actually run them.
#[must_use]
pub fn help_topics(grep_enabled: bool, pickaxe_enabled: bool) -> Vec<SearchHelpTopic> {
    let mut topics: Vec<SearchHelpTopic> = vec![SearchHelpTopic::Commit];
    if grep_enabled {
        topics.push(SearchHelpTopic::Grep);
    }
    topics.push(SearchHelpTopic::Author);
    topics.push(SearchHelpTopic::Committer);
    if pickaxe_enabled {
        topics.push(SearchHelpTopic::Pickaxe);
    }
    topics
}
