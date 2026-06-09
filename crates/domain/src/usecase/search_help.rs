//! The search-help use case (gitweb's `git_search_help`).
//!
//! gitweb's search-help page documents the search types, gating the `grep` and
//! `pickaxe` entries behind their features. The page itself reads no repository
//! object — it is pure help prose — so this Control has no [`Repository`] port to
//! drive: it only reads the `grep` and `pickaxe` feature gates from [`Settings`]
//! and applies the [`help_topics`] rule, yielding the ordered topic list the
//! render layer turns into the help document.
//!
//! [`Repository`]: crate::port::repository::Repository

use crate::model::search_help::{SearchHelpTopic, help_topics};
use crate::model::settings::{FeatureName, Settings};

/// The assembled search-help page: the ordered search-type topics to document,
/// already filtered by the site's `grep`/`pickaxe` feature configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHelpView {
    topics: Vec<SearchHelpTopic>,
}

impl SearchHelpView {
    /// The topics to document, in gitweb's fixed order.
    #[must_use]
    pub fn topics(&self) -> &[SearchHelpTopic] {
        &self.topics
    }
}

/// Assembles the search-help page from the site `settings`. Reads the `grep` and
/// `pickaxe` feature gates and applies the [`help_topics`] rule; the always-on
/// `commit`/`author`/`committer` topics are documented regardless, so this never
/// fails and needs no repository access.
#[must_use]
pub fn assemble_search_help(settings: &Settings) -> SearchHelpView {
    let grep_enabled: bool = settings.feature(FeatureName::Grep).enabled();
    let pickaxe_enabled: bool = settings.feature(FeatureName::Pickaxe).enabled();
    SearchHelpView {
        topics: help_topics(grep_enabled, pickaxe_enabled),
    }
}
