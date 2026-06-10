//! The `summary` use case: gitweb's `git_summary`, minus the HTML.
//!
//! Summary is gitweb's landing page and default action. It composes one
//! overview from pieces every other capability already produces: a metadata
//! block (description, owner, last-change time of HEAD, and the project's clone
//! URLs), an optional raw `README.html`, then the recent shortlog, the tags, and
//! the heads — each capped at gitweb's 16 with a "..." link to its full action
//! when it overran (the [`Section`] rule). Remotes and forks are gated behind
//! their own features and land with those capabilities.
//!
//! This is the orchestration over the [`Repository`] port plus the per-project
//! metadata the boundary read from the [`ProjectStore`](crate::port::project_store::ProjectStore)
//! (the [`ProjectInfo`] and the README bytes). It reuses the shortlog, tags, and
//! heads use cases wholesale rather than re-reading refs, so their ordering and
//! age rules apply here unchanged. The clock is injected (`now`) for the section
//! ages, the same discipline as those use cases.
//!
//! Two settings shape the page, exactly as gitweb gates them: `omit_owner`
//! suppresses the owner row, and `prevent_xss` suppresses the README (the raw
//! sink gitweb refuses to emit under XSS prevention). An unborn repository (no
//! HEAD) shows its metadata and refs but no last-change time and an empty
//! shortlog, and is never an error.

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::project_info::ProjectInfo;
use crate::model::section::Section;
use crate::model::settings::Settings;
use crate::model::timestamp::Timestamp;
use crate::port::repository::{Page, Repository};
use crate::usecase::heads::{HeadRow, assemble_heads};
use crate::usecase::shortlog::{ShortlogView, assemble_shortlog};
use crate::usecase::tags::{TagRow, assemble_tags};

/// gitweb's summary cap: each of the shortlog, tags, and heads sections shows at
/// most this many before offering a "..." link (`git_get_*_list(16)`).
const SUMMARY_LIMIT: usize = 16;

/// gitweb's `git_get_project_description($project) || "none"` placeholder.
const NO_DESCRIPTION: &str = "none";

/// The assembled summary page: the metadata block, the optional README, and the
/// three capped sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryView {
    description: String,
    owner: Option<String>,
    last_change: Option<Timestamp>,
    clone_urls: Vec<String>,
    readme: Option<String>,
    shortlog: ShortlogView,
    tags: Section<TagRow>,
    heads: Section<HeadRow>,
}

impl SummaryView {
    /// The project description, or gitweb's `none` placeholder when it has none.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The owner to show, or `None` when the project has none or `omit_owner` is
    /// on.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// The absolute last-change time — HEAD's committer timestamp — or `None` for
    /// an unborn repository with no HEAD commit (gitweb shows no "last change").
    #[must_use]
    pub fn last_change(&self) -> Option<&Timestamp> {
        self.last_change.as_ref()
    }

    /// The clone URLs to advertise: the project's own list, or the base-URL +
    /// name fallback when it has none.
    #[must_use]
    pub fn clone_urls(&self) -> &[String] {
        &self.clone_urls
    }

    /// The raw README HTML to inject, or `None` when there is none or XSS
    /// prevention suppressed it. A DELIBERATE raw-HTML sink — the render layer
    /// emits it verbatim.
    #[must_use]
    pub fn readme(&self) -> Option<&str> {
        self.readme.as_deref()
    }

    /// The recent-history section (gitweb's `git_shortlog_body` window).
    #[must_use]
    pub fn shortlog(&self) -> &ShortlogView {
        &self.shortlog
    }

    /// The tags section, capped with its overflow flag.
    #[must_use]
    pub fn tags(&self) -> &Section<TagRow> {
        &self.tags
    }

    /// The heads section, capped with its overflow flag.
    #[must_use]
    pub fn heads(&self) -> &Section<HeadRow> {
        &self.heads
    }
}

/// Assembles the summary page over the [`Repository`] port and the per-project
/// metadata the boundary resolved (`info` and the optional `readme` bytes).
/// `settings` carries the `omit_owner` and `prevent_xss` gates and the clone-URL
/// base list; `now` is the request-time epoch the section ages are measured
/// against.
///
/// # Errors
///
/// Propagates the repository's error if reading HEAD's commit, the history, or a
/// ref listing fails. An unborn repository whose HEAD names no commit is not an
/// error — it yields no last-change time and an empty shortlog.
pub fn assemble_summary(
    repo: &dyn Repository,
    info: &ProjectInfo,
    readme: Option<&str>,
    settings: &Settings,
    now: i64,
) -> Result<SummaryView, DomainError> {
    let last_change: Option<Timestamp> =
        head_commit(repo)?.map(|commit: Commit| Timestamp::from_signature(commit.committer()));
    let shortlog: ShortlogView = assemble_shortlog(repo, None, now, Page::new(0, SUMMARY_LIMIT))?;
    let tags: Section<TagRow> =
        Section::limited(assemble_tags(repo, now)?.rows().to_vec(), SUMMARY_LIMIT);
    // TODO(d7s): thread the resolved get_branch_refs through here so the summary
    // heads section honours extra-branch-refs; heads-only until that commit.
    let heads: Section<HeadRow> = Section::limited(
        assemble_heads(repo, now, &["heads"])?.rows().to_vec(),
        SUMMARY_LIMIT,
    );
    Ok(SummaryView {
        description: described(info),
        owner: shown_owner(info, settings),
        last_change,
        clone_urls: clone_urls(info, settings),
        readme: included_readme(readme, settings),
        shortlog,
        tags,
        heads,
    })
}

/// HEAD's commit, or `None` for an unborn repository whose HEAD names no commit
/// (gitweb's empty `parse_commit("HEAD")`). Any other failure propagates.
fn head_commit(repo: &dyn Repository) -> Result<Option<Commit>, DomainError> {
    match repo.head() {
        Ok(reference) => Ok(Some(repo.find_commit(reference.target())?)),
        Err(DomainError::NotFound(_)) => Ok(None),
        Err(other) => Err(other),
    }
}

/// The description to show, or gitweb's `none` for an absent or empty one
/// (gitweb's `|| "none"`, which an empty string also triggers).
fn described(info: &ProjectInfo) -> String {
    match info.description() {
        Some(text) if !text.is_empty() => text.to_owned(),
        _ => NO_DESCRIPTION.to_owned(),
    }
}

/// The owner to show: the project's owner unless it is empty or `omit_owner` is
/// on (gitweb's `if ($owner and not $omit_owner)`).
fn shown_owner(info: &ProjectInfo, settings: &Settings) -> Option<String> {
    if settings.omit_owner() {
        return None;
    }
    info.owner()
        .filter(|owner: &&str| !owner.is_empty())
        .map(str::to_owned)
}

/// The clone URLs to advertise: the project's own non-empty URLs, or — when it
/// has none — the `<base>/<project>` fallback over the configured base URLs
/// (gitweb's `@url_list = map { "$_/$project" } @git_base_url_list unless
/// @url_list`, with its `next unless $git_url` dropping empties).
fn clone_urls(info: &ProjectInfo, settings: &Settings) -> Vec<String> {
    let own: Vec<String> = info
        .clone_urls()
        .iter()
        .filter(|url: &&String| !url.is_empty())
        .cloned()
        .collect();
    if !own.is_empty() {
        return own;
    }
    settings
        .git_base_url_list()
        .iter()
        .map(|base: &String| format!("{base}/{}", info.name()))
        .filter(|url: &String| !url.is_empty())
        .collect()
}

/// The README to inject, suppressed when XSS prevention is on (gitweb's
/// `if (!$prevent_xss && -s README.html)`; the `-s` non-empty test is the
/// store's, so a present `readme` is already non-empty).
fn included_readme(readme: Option<&str>, settings: &Settings) -> Option<String> {
    if settings.prevent_xss() {
        return None;
    }
    readme.map(str::to_owned)
}
