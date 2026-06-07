//! Request actions: gitweb's `%actions` dispatch table as a domain entity.
//!
//! Every gitweb request resolves to one action — the key that selects a view.
//! [`Action::parse`] mirrors gitweb's `is_valid_action` (an unknown name is no
//! action, which the web boundary turns into `die_error(400, ...)`), and
//! [`Action::needs_project`] mirrors the `%actions` split between the three
//! entries gitweb comments as "those below don't need $project" and the rest.

/// One entry in gitweb's `%actions` dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// `blame` — line-by-line authorship of a file.
    Blame,
    /// `blame_incremental` — the progressive-blame host page.
    BlameIncremental,
    /// `blame_data` — the blame data feed the incremental view consumes.
    BlameData,
    /// `blobdiff` — diff of a single file between two revisions.
    Blobdiff,
    /// `blobdiff_plain` — raw unified diff of a single file.
    BlobdiffPlain,
    /// `blob` — a single file's contents.
    Blob,
    /// `blob_plain` — a single file served raw.
    BlobPlain,
    /// `commitdiff` — diff introduced by a commit.
    Commitdiff,
    /// `commitdiff_plain` — raw unified diff of a commit.
    CommitdiffPlain,
    /// `commit` — a single commit's metadata and diff.
    Commit,
    /// `forks` — repositories forked from this one.
    Forks,
    /// `heads` — the branch heads.
    Heads,
    /// `history` — a file's commit history.
    History,
    /// `log` — the commit log with full messages.
    Log,
    /// `patch` — a single commit as a mailbox patch.
    Patch,
    /// `patches` — a range of commits as mailbox patches.
    Patches,
    /// `remotes` — configured remotes.
    Remotes,
    /// `rss` — the commit feed as RSS.
    Rss,
    /// `atom` — the commit feed as Atom.
    Atom,
    /// `search` — commit/content/author search results.
    Search,
    /// `search_help` — the search syntax help page.
    SearchHelp,
    /// `shortlog` — the commit log, one line each.
    Shortlog,
    /// `summary` — a project's landing page.
    Summary,
    /// `tag` — a single annotated tag.
    Tag,
    /// `tags` — the tag list.
    Tags,
    /// `tree` — a directory listing.
    Tree,
    /// `snapshot` — a downloadable archive of a revision.
    Snapshot,
    /// `object` — dispatch to the view for an object's kind.
    Object,
    /// `opml` — the project list as OPML (no project).
    Opml,
    /// `project_list` — the list of projects (no project).
    ProjectList,
    /// `project_index` — the machine-readable project index (no project).
    ProjectIndex,
}

impl Action {
    /// Parses gitweb's wire action name, returning `None` for any name that is
    /// not a key in the `%actions` table — gitweb's `is_valid_action`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "blame" => Some(Self::Blame),
            "blame_incremental" => Some(Self::BlameIncremental),
            "blame_data" => Some(Self::BlameData),
            "blobdiff" => Some(Self::Blobdiff),
            "blobdiff_plain" => Some(Self::BlobdiffPlain),
            "blob" => Some(Self::Blob),
            "blob_plain" => Some(Self::BlobPlain),
            "commitdiff" => Some(Self::Commitdiff),
            "commitdiff_plain" => Some(Self::CommitdiffPlain),
            "commit" => Some(Self::Commit),
            "forks" => Some(Self::Forks),
            "heads" => Some(Self::Heads),
            "history" => Some(Self::History),
            "log" => Some(Self::Log),
            "patch" => Some(Self::Patch),
            "patches" => Some(Self::Patches),
            "remotes" => Some(Self::Remotes),
            "rss" => Some(Self::Rss),
            "atom" => Some(Self::Atom),
            "search" => Some(Self::Search),
            "search_help" => Some(Self::SearchHelp),
            "shortlog" => Some(Self::Shortlog),
            "summary" => Some(Self::Summary),
            "tag" => Some(Self::Tag),
            "tags" => Some(Self::Tags),
            "tree" => Some(Self::Tree),
            "snapshot" => Some(Self::Snapshot),
            "object" => Some(Self::Object),
            "opml" => Some(Self::Opml),
            "project_list" => Some(Self::ProjectList),
            "project_index" => Some(Self::ProjectIndex),
            _ => None,
        }
    }

    /// The wire action name (the `%actions` key), the inverse of [`Self::parse`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blame => "blame",
            Self::BlameIncremental => "blame_incremental",
            Self::BlameData => "blame_data",
            Self::Blobdiff => "blobdiff",
            Self::BlobdiffPlain => "blobdiff_plain",
            Self::Blob => "blob",
            Self::BlobPlain => "blob_plain",
            Self::Commitdiff => "commitdiff",
            Self::CommitdiffPlain => "commitdiff_plain",
            Self::Commit => "commit",
            Self::Forks => "forks",
            Self::Heads => "heads",
            Self::History => "history",
            Self::Log => "log",
            Self::Patch => "patch",
            Self::Patches => "patches",
            Self::Remotes => "remotes",
            Self::Rss => "rss",
            Self::Atom => "atom",
            Self::Search => "search",
            Self::SearchHelp => "search_help",
            Self::Shortlog => "shortlog",
            Self::Summary => "summary",
            Self::Tag => "tag",
            Self::Tags => "tags",
            Self::Tree => "tree",
            Self::Snapshot => "snapshot",
            Self::Object => "object",
            Self::Opml => "opml",
            Self::ProjectList => "project_list",
            Self::ProjectIndex => "project_index",
        }
    }

    /// Whether serving this action requires a project. Gitweb serves
    /// `project_list`, `project_index` and `opml` without one; everything else
    /// needs a project.
    #[must_use]
    pub fn needs_project(self) -> bool {
        !matches!(self, Self::Opml | Self::ProjectList | Self::ProjectIndex)
    }
}
