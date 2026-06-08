//! The `rss` / `atom` handlers: gitweb's `git_feed`.
//!
//! Glue only. It opens the requested project, runs the [`assemble_feed`] use case
//! over it (HEAD by default, narrowed to a file when one is named), maps the
//! structured [`Feed`] into the render layer's [`FeedView`] — building every
//! absolute link with [`href_full`], since URLs are the boundary's job — and
//! serializes it as RSS or Atom. The single handler serves both actions; the
//! request's action picks the format.
//!
//! The `Last-Modified` header is emitted from the newest commit's date here; the
//! conditional-GET `304` it enables, and the `Accept`-driven `text/xml` fallback,
//! both need request headers the handler does not yet see and live with the
//! caching cross-cut. The request-time clock is read at the boundary so the feed
//! window stays computed against a real `now` while the domain stays clock-free.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::feed::{Feed, FeedEntry, FeedFile, feed_title};
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::feed::assemble_feed;
use gitweb_render::feed::{FeedEntryView, FeedFileView, FeedView, atom, rss};

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::{href_full, self_url};

/// Which syndication format a feed request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedFormat {
    /// `rss` — RSS 2.0.
    Rss,
    /// `atom` — Atom 1.0.
    Atom,
}

impl FeedFormat {
    /// The action wire name (`rss` / `atom`), used in the feed title and URLs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rss => "rss",
            Self::Atom => "atom",
        }
    }

    /// The feed media type without charset — the Atom self-link `type`.
    #[must_use]
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Rss => "application/rss+xml",
            Self::Atom => "application/atom+xml",
        }
    }

    /// The `Content-Type` header value gitweb sends (`-charset => 'utf-8'`).
    #[must_use]
    pub fn content_type_header(self) -> &'static str {
        match self {
            Self::Rss => "application/rss+xml; charset=utf-8",
            Self::Atom => "application/atom+xml; charset=utf-8",
        }
    }

    /// The word in the missing-description fallback (`$project RSS/Atom feed`).
    fn descr_word(self) -> &'static str {
        match self {
            Self::Rss => "RSS",
            Self::Atom => "Atom",
        }
    }

    /// The format the request's action names (`atom`, else `rss`).
    fn of(request: &Request) -> Self {
        match request.action {
            Some(Action::Atom) => Self::Atom,
            _ => Self::Rss,
        }
    }
}

/// The request facts a feed view is built from: which format, which project, and
/// the optional branch and file it is scoped to.
#[derive(Debug, Clone, Copy)]
pub struct FeedRequest<'a> {
    /// The syndication format.
    pub format: FeedFormat,
    /// The project the feed is for.
    pub project: &'a str,
    /// The branch/revision, or `None` for HEAD.
    pub hash: Option<&'a str>,
    /// The file the feed is the history of, or `None` for the whole branch.
    pub file_name: Option<&'a str>,
}

/// Maps the domain [`Feed`] and the project metadata into the render layer's
/// [`FeedView`], building every absolute link with [`href_full`]. `base` is the
/// site URL (scheme and host, no trailing slash); `generator` is the
/// `version/git_version` composite gitweb stamps the feed with. Shared by the
/// handler and the golden conformance test so both produce identical output.
#[must_use]
pub fn assemble_feed_view(
    feed: &Feed,
    request: &FeedRequest<'_>,
    info: &ProjectInfo,
    settings: &Settings,
    base: &str,
    generator: &str,
) -> FeedView {
    let project: &str = request.project;
    let action: &str = request.format.as_str();
    let blame_on: bool = settings.feature(FeatureName::Blame).enabled();
    let entries: Vec<FeedEntryView> = feed
        .entries()
        .iter()
        .map(|entry: &FeedEntry| entry_view(entry, request, base, blame_on))
        .collect();
    FeedView {
        title: feed_title(
            settings.site_name(),
            project,
            action,
            request.hash,
            request.file_name,
        ),
        description: info.description().map_or_else(
            || format!("{project} {} feed", request.format.descr_word()),
            str::to_owned,
        ),
        owner: info.owner().unwrap_or_default().to_owned(),
        generator: generator.to_owned(),
        logo: settings.logo().to_owned(),
        favicon: settings.favicon().to_owned(),
        alt_url: alt_url(request, base),
        id_url: href_full(base, &[("p", project)]),
        self_url: self_url(base, &self_params(request)),
        xml_base: format!("{base}/"),
        content_type: request.format.media_type().to_owned(),
        latest: feed.latest().cloned(),
        entries,
    }
}

/// The HTML page this feed mirrors (gitweb's `$alt_url`): the file's history, the
/// branch log, or the project summary.
fn alt_url(request: &FeedRequest<'_>, base: &str) -> String {
    let project: &str = request.project;
    match (request.hash, request.file_name) {
        (hash, Some(file)) => {
            let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", "history")];
            if let Some(hash) = hash {
                params.push(("h", hash));
            }
            params.push(("f", file));
            href_full(base, &params)
        }
        (Some(hash), None) => href_full(base, &[("p", project), ("a", "log"), ("h", hash)]),
        (None, None) => href_full(base, &[("p", project), ("a", "summary")]),
    }
}

/// The request's own parameters, for the Atom self link (gitweb's `self_url()`).
fn self_params<'a>(request: &FeedRequest<'a>) -> Vec<(&'a str, &'a str)> {
    let mut params: Vec<(&str, &str)> =
        vec![("p", request.project), ("a", request.format.as_str())];
    if let Some(hash) = request.hash {
        params.push(("h", hash));
    }
    if let Some(file) = request.file_name {
        params.push(("f", file));
    }
    params
}

/// Maps one feed entry to its render view, building the per-commit and per-file
/// links.
fn entry_view(
    entry: &FeedEntry,
    request: &FeedRequest<'_>,
    base: &str,
    blame_on: bool,
) -> FeedEntryView {
    let project: &str = request.project;
    let id: &str = entry.id();
    let files: Vec<FeedFileView> = entry
        .files()
        .iter()
        .map(|file: &FeedFile| file_view(file, entry, request, base, blame_on))
        .collect();
    FeedEntryView {
        title: entry.title().to_owned(),
        author_full: entry.author_full().to_owned(),
        author_name: entry.author_name().to_owned(),
        author_email: entry.author_email().map(str::to_owned),
        committer_name: entry.committer_name().to_owned(),
        committer_email: entry.committer_email().map(str::to_owned),
        timestamp: entry.timestamp().clone(),
        commitdiff_url: href_full(base, &[("p", project), ("a", "commitdiff"), ("h", id)]),
        comment: entry.comment().to_vec(),
        files,
    }
}

/// Maps one changed file to its render view: the `blobdiff` link always, the
/// `blame` link when the feature is on, and the `history` link except for the
/// very file a file-history feed is about (gitweb's per-link conditions).
fn file_view(
    file: &FeedFile,
    entry: &FeedEntry,
    request: &FeedRequest<'_>,
    base: &str,
    blame_on: bool,
) -> FeedFileView {
    let project: &str = request.project;
    let to_path: &str = file.to_path();
    let id: &str = entry.id();
    let mut blobdiff: Vec<(&str, &str)> = vec![
        ("p", project),
        ("a", "blobdiff"),
        ("f", to_path),
        ("fp", file.from_path()),
        ("h", file.to_oid()),
        ("hp", file.from_oid()),
        ("hb", id),
    ];
    if let Some(parent) = entry.parent() {
        blobdiff.push(("hpb", parent));
    }
    let blame_url: Option<String> = blame_on.then(|| {
        href_full(
            base,
            &[("p", project), ("a", "blame"), ("f", to_path), ("hb", id)],
        )
    });
    let show_history: bool = request.file_name != Some(to_path);
    let history_url: Option<String> = show_history.then(|| {
        href_full(
            base,
            &[("p", project), ("a", "history"), ("f", to_path), ("h", id)],
        )
    });
    FeedFileView {
        to_path: to_path.to_owned(),
        blobdiff_url: href_full(base, &blobdiff),
        blame_url,
        history_url,
    }
}

/// Serves the RSS/Atom feed over a wired project store and resolved settings,
/// stamped with the configured site `base` URL and feed `generator`.
pub struct FeedHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
    base: String,
    generator: String,
}

impl FeedHandler {
    /// Wires the handler with the store it opens projects from, the settings it
    /// reads, the site `base` URL feed links are absolute against, and the feed
    /// `generator` version string.
    #[must_use]
    pub fn new(
        store: Arc<dyn ProjectStore + Send + Sync>,
        settings: Arc<Settings>,
        base: String,
        generator: String,
    ) -> Self {
        Self {
            store,
            settings,
            base,
            generator,
        }
    }
}

impl Handler for FeedHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let format: FeedFormat = FeedFormat::of(request);
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let info: ProjectInfo = self.store.info(project)?;
        let rev: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let path: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let feed: Feed = assemble_feed(repository.as_ref(), rev, path, now_epoch())?;
        let last_modified: Option<String> = feed.latest().map(Timestamp::rfc2822);
        let feed_request: FeedRequest<'_> = FeedRequest {
            format,
            project,
            hash: rev,
            file_name: path,
        };
        let view: FeedView = assemble_feed_view(
            &feed,
            &feed_request,
            &info,
            &self.settings,
            &self.base,
            &self.generator,
        );
        let body: String = match format {
            FeedFormat::Rss => rss(&view),
            FeedFormat::Atom => atom(&view),
        };
        Ok(View::feed(
            format.content_type_header(),
            body,
            last_modified,
        ))
    }
}
