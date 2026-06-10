//! [`GixProjectStore`]: the gix-backed [`ProjectStore`] adapter.
//!
//! Discovers the repositories under a project root and opens one by name,
//! mirroring gitweb's `git_get_projects_list` (directory walk) and
//! `is_valid_project` (validate the name, then confirm the directory is a git
//! repository). A directory is a repository when gitweb's `check_head_link`
//! holds — a `HEAD` is present, or is a symlink into `refs/heads/`. As in
//! gitweb, discovery does not descend into a repository once found, so the
//! shallowest repository on each branch of the tree wins. Export gates
//! (`$export_ok`, `$strict_export`, the auth hook) and per-project metadata are
//! a later slice and are deliberately not consulted here.

use std::fs;
use std::path::{Path, PathBuf};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::branch_refs::get_branch_refs;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_filter::ProjectFilter;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::projects_list::{ProjectListEntry, parse_project_line};
use gitweb_domain::model::safety::SafePath;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;

use crate::conv::{backend, to_signature};
use crate::repository::GixRepository;
use crate::user_directory::{SystemUserDirectory, UserDirectory};

/// Discovery and opening of the repositories under one project root, backed by
/// the filesystem and gix.
///
/// gitweb lists projects in one of two ways (`git_get_projects_list`): by
/// scanning `$projectroot` for repositories, or by reading a `$projects_list`
/// file that names each project (and optionally its owner). `list_file` selects
/// between them; either way, projects are opened and read relative to `root`.
#[derive(Debug)]
pub struct GixProjectStore {
    root: PathBuf,
    list_file: Option<PathBuf>,
    /// Resolves the filesystem owner of a repository directory (gitweb's
    /// `get_file_owner`), the last-resort owner source. Defaults to the system
    /// passwd database; a conformance spec injects a pinned one via
    /// [`with_user_directory`](Self::with_user_directory).
    users: Box<dyn UserDirectory>,
    /// The configured `extra-branch-refs` feature options (gitweb's
    /// `@extra_branch_refs`, the raw entries — one ref directory each), resolved
    /// to the full branch-directory set by [`get_branch_refs`] when last activity
    /// is computed. Empty by default (heads-only); the composition root injects
    /// the deployment's value via [`with_extra_branch_refs`](Self::with_extra_branch_refs).
    extra_branch_refs: Vec<String>,
}

impl GixProjectStore {
    /// A store that discovers projects by scanning `root` (the `$projectroot`).
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            list_file: None,
            users: Box::new(SystemUserDirectory),
            extra_branch_refs: Vec::new(),
        }
    }

    /// A store that lists projects from a `$projects_list` file, with their
    /// paths resolved (and opened) relative to `root`.
    #[must_use]
    pub fn from_list_file(root: PathBuf, list_file: PathBuf) -> Self {
        Self {
            root,
            list_file: Some(list_file),
            users: Box::new(SystemUserDirectory),
            extra_branch_refs: Vec::new(),
        }
    }

    /// Replaces the user directory used for the filesystem owner fallback, so a
    /// test can pin the uid → name mapping that is otherwise the host's
    /// non-reproducible passwd database.
    #[must_use]
    pub fn with_user_directory(mut self, users: Box<dyn UserDirectory>) -> Self {
        self.users = users;
        self
    }

    /// Sets the `extra-branch-refs` feature options (gitweb's `@extra_branch_refs`)
    /// so the last-activity scan spans every branch directory `get_branch_refs`
    /// reports, not just `refs/heads/`. The composition root resolves the option
    /// list from the global settings and injects it here.
    #[must_use]
    pub fn with_extra_branch_refs(mut self, extra_branch_refs: Vec<String>) -> Self {
        self.extra_branch_refs = extra_branch_refs;
        self
    }

    /// Resolves a store-relative `name` to its on-disk git directory, validating
    /// the name first (gitweb's `is_valid_project`) so the join can never escape
    /// the root, and confirming a repository actually lives there.
    fn locate(&self, name: &str) -> Result<PathBuf, DomainError> {
        let safe: SafePath =
            SafePath::parse(name).ok_or_else(|| DomainError::Invalid(name.to_owned()))?;
        let path: PathBuf = self.root.join(safe.as_str());
        if !head_linked(&path) {
            return Err(DomainError::NotFound(name.to_owned()));
        }
        Ok(path)
    }

    /// Lists the projects named in the `$projects_list` file: each parsed line
    /// whose path is a safe, existing repository (gitweb's file-mode
    /// `git_get_projects_list`, whose `check_export_ok` requires `check_head_link`
    /// — export markers and the auth hook are a later slice). File order is
    /// preserved, as gitweb keeps it.
    fn list_from_file(&self, file: &Path) -> Result<Vec<Project>, DomainError> {
        let text: String = fs::read_to_string(file).map_err(backend)?;
        let mut found: Vec<Project> = Vec::new();
        for line in text.lines() {
            let Some(entry) = parse_project_line(line) else {
                continue;
            };
            // Validate the path before touching the filesystem, then keep it only
            // if a repository really lives there.
            let Some(safe) = SafePath::parse(entry.path()) else {
                continue;
            };
            if head_linked(&self.root.join(safe.as_str())) {
                found.push(Project::new(entry.path().to_owned()));
            }
        }
        Ok(found)
    }

    /// The owner named for `name` in the `$projects_list` file, if the store is
    /// in file mode and the file carries one (gitweb consults this before the
    /// `gitweb.owner` config value).
    fn list_owner(&self, name: &str) -> Option<String> {
        let file: &PathBuf = self.list_file.as_ref()?;
        let text: String = fs::read_to_string(file).ok()?;
        text.lines()
            .filter_map(parse_project_line)
            .find(|entry: &ProjectListEntry| entry.path() == name)
            .and_then(|entry: ProjectListEntry| entry.owner().map(str::to_owned))
    }

    /// gitweb's `get_file_owner` fallback: the display name of the user that owns
    /// the repository directory on disk. `stat`s the directory for its owning
    /// uid (following symlinks, as gitweb's `stat` does) and resolves that uid
    /// through the [`UserDirectory`] seam. `None` when the directory cannot be
    /// stat'd or the uid has no passwd entry, so the owner is simply absent.
    fn file_owner(&self, git_dir: &Path) -> Option<String> {
        use std::os::unix::fs::MetadataExt;
        let uid: u32 = fs::metadata(git_dir).ok()?.uid();
        self.users.display_name(uid)
    }
}

impl ProjectStore for GixProjectStore {
    fn list(&self, filter: Option<&ProjectFilter>) -> Result<Vec<Project>, DomainError> {
        let mut found: Vec<Project> = if let Some(file) = &self.list_file {
            self.list_from_file(file)?
        } else {
            let mut scanned: Vec<Project> = Vec::new();
            scan(&self.root, &self.root, &mut scanned)?;
            // gitweb leaves order to File::Find; sort for a deterministic listing.
            scanned.sort_by(|a: &Project, b: &Project| a.name().cmp(b.name()));
            scanned
        };
        // gitweb's $project_filter: keep only the projects under the subdirectory
        // (file mode's `^\Qfilter\E/`, equivalent to the dir walk's scoped root).
        if let Some(filter) = filter {
            found.retain(|project: &Project| filter.include(project.name()));
        }
        Ok(found)
    }

    fn open(&self, name: &str) -> Result<Box<dyn Repository>, DomainError> {
        // Validate before access: reject traversal, absolute and NUL-bearing
        // names so the join can never escape the root (gitweb is_valid_project),
        // and confirm a repository really lives there.
        let path: PathBuf = self.locate(name)?;
        let repo: GixRepository = GixRepository::open(&path)?;
        Ok(Box::new(repo))
    }

    fn container_exists(&self, subdir: &str) -> bool {
        // gitweb's `-d "$projectroot/$path"`. Validate the name first so the join
        // can never escape the root — an unsafe name is simply not a container —
        // then test that the join is a directory (following symlinks, as Perl's
        // `-d` does). Any non-directory, missing, or unreadable path answers
        // false, never an error.
        SafePath::parse(subdir).is_some_and(|safe: SafePath| self.root.join(safe.as_str()).is_dir())
    }

    fn info(&self, name: &str) -> Result<ProjectInfo, DomainError> {
        let git_dir: PathBuf = self.locate(name)?;
        let repo: gix::Repository = gix::open(&git_dir).map_err(backend)?;
        let config: gix::config::Snapshot<'_> = repo.config_snapshot();

        // Each scalar field is gitweb's git_get_file_or_project_config: the
        // repository file if present, otherwise the gitweb.<key> config value.
        let mut info: ProjectInfo = ProjectInfo::named(name);
        if let Some(description) =
            file_or_config(&git_dir, "description", &config, "gitweb.description")
        {
            info = info.with_description(description);
        }
        // Owner precedence (gitweb's git_get_project_owner): the projects-list
        // file wins, then the gitweb.owner config value, and only as a last
        // resort the operating-system owner of the directory (get_file_owner).
        if let Some(owner) = self
            .list_owner(name)
            .or_else(|| config.string("gitweb.owner").map(cow_to_string))
            .or_else(|| self.file_owner(&git_dir))
        {
            info = info.with_owner(owner);
        }
        if let Some(category) = file_or_config(&git_dir, "category", &config, "gitweb.category") {
            info = info.with_category(category);
        }
        for url in clone_urls(&git_dir, &config) {
            info = info.with_clone_url(url);
        }
        // gitweb's git_get_last_activity: the committer time of the most recent
        // branch, scanned over `map { "refs/$_" } get_branch_refs()` — heads plus
        // the validated extra-branch-refs directories (a malformed entry is
        // gitweb's die_error(500), surfaced here). Absent for an unborn/empty
        // repo, so the field stays unset.
        let branch_refs: Vec<String> = get_branch_refs(&self.extra_branch_refs)?;
        if let Some(when) = most_recent_branch_time(&repo, &branch_refs)? {
            info = info.with_last_activity(when);
        }
        Ok(info)
    }

    fn readme_html(&self, name: &str) -> Result<Option<String>, DomainError> {
        // Validate the name and confirm a repository lives there first, so a
        // bad name fails like open()/info() rather than silently reading no
        // README.
        let git_dir: PathBuf = self.locate(name)?;
        Ok(read_nonempty(&git_dir.join("README.html")))
    }

    fn description(&self, name: &str) -> Result<Option<String>, DomainError> {
        // gitweb's git_get_project_description: the $GIT_DIR/description file,
        // else the gitweb.description config value. The same file_or_config read
        // info() uses, but on its own — no owner/category/clone/last-activity
        // work. locate() validates the name and confirms a repository lives
        // there, so a bad name fails like open()/info().
        let git_dir: PathBuf = self.locate(name)?;
        let repo: gix::Repository = gix::open(&git_dir).map_err(backend)?;
        let config: gix::config::Snapshot<'_> = repo.config_snapshot();
        Ok(file_or_config(
            &git_dir,
            "description",
            &config,
            "gitweb.description",
        ))
    }
}

/// gitweb's `-s "$projectroot/$project/README.html"`: the file's bytes when it
/// exists and is non-empty, decoded lossily (it is HTML, normally UTF-8), or
/// `None` when the file is absent or empty. A read error other than absence —
/// a permission failure, say — also yields `None`, the same way gitweb's `-s`
/// test simply fails the condition rather than dying.
fn read_nonempty(path: &Path) -> Option<String> {
    let bytes: Vec<u8> = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// gitweb's `git_get_last_activity`: the committer timestamp (Unix epoch seconds)
/// of the project's most recently updated branch. gitweb runs `git for-each-ref
/// --sort=-committerdate --count=1` over `map { "refs/$_" } get_branch_refs()`
/// and reads the committer time off the winning ref; here we take the maximum
/// committer time over the refs under every branch directory in `branch_refs`
/// (`heads`, plus the validated extra-branch-refs entries). Tags and other refs
/// are not branches, so they never count. A repository with no branch commits —
/// unborn or empty — has no activity, so the result is `None`.
fn most_recent_branch_time(
    repo: &gix::Repository,
    branch_refs: &[String],
) -> Result<Option<i64>, DomainError> {
    let platform: gix::reference::iter::Platform<'_> = repo.references().map_err(backend)?;
    let mut latest: Option<i64> = None;
    for item in platform.all().map_err(backend)? {
        let reference: gix::Reference<'_> = item.map_err(backend)?;
        let full: String = reference.name().as_bstr().to_string();
        if !is_branch_ref(&full, branch_refs) {
            continue;
        }
        let commit: gix::Commit<'_> = reference
            .into_fully_peeled_id()
            .map_err(backend)?
            .object()
            .map_err(backend)?
            .try_into_commit()
            .map_err(|_error: gix::object::try_into::Error| {
                backend("a branch ref did not resolve to a commit")
            })?;
        let signature: gix::actor::SignatureRef<'_> = commit.committer().map_err(backend)?;
        let when: i64 = to_signature(signature)?.epoch();
        latest = Some(latest.map_or(when, |best: i64| best.max(when)));
    }
    Ok(latest)
}

/// Whether the fully-qualified ref `full` lives directly under one of the branch
/// directories in `branch_refs` — gitweb's `map { "refs/$_/" } get_branch_refs()`
/// membership test. `refs/heads/main` with `heads` listed matches; a tag or a ref
/// under an unlisted directory does not.
fn is_branch_ref(full: &str, branch_refs: &[String]) -> bool {
    branch_refs
        .iter()
        .any(|dir: &String| full.starts_with(&format!("refs/{dir}/")))
}

/// gitweb's `git_get_file_or_project_config`: the first line of the repository
/// file `file_name`, or, when that file is absent, the `gitweb.<key>` config
/// value.
fn file_or_config(
    git_dir: &Path,
    file_name: &str,
    config: &gix::config::Snapshot<'_>,
    key: &str,
) -> Option<String> {
    match first_line(&git_dir.join(file_name)) {
        Some(line) => Some(line),
        None => config.string(key).map(cow_to_string),
    }
}

/// The clone URLs: every line of the repository's `cloneurl` file, or, when that
/// file is absent, the (possibly multi-valued) `gitweb.url` config — matching
/// gitweb's `git_get_project_url_list`.
fn clone_urls(git_dir: &Path, config: &gix::config::Snapshot<'_>) -> Vec<String> {
    if let Ok(text) = fs::read_to_string(git_dir.join("cloneurl")) {
        return text.lines().map(str::to_owned).collect();
    }
    config
        .plumbing()
        .strings("gitweb.url")
        .map(|values: Vec<std::borrow::Cow<'_, gix::bstr::BStr>>| {
            values.into_iter().map(cow_to_string).collect()
        })
        .unwrap_or_default()
}

/// The first line of `path` with its trailing newline removed (gitweb reads one
/// line and `chomp`s it). `None` when the file is missing or empty, so the
/// caller falls through to the config value.
fn first_line(path: &Path) -> Option<String> {
    let text: String = fs::read_to_string(path).ok()?;
    text.lines().next().map(str::to_owned)
}

/// A git config value as an owned `String`, decoded lossily from its raw bytes.
fn cow_to_string(value: std::borrow::Cow<'_, gix::bstr::BStr>) -> String {
    value.to_string()
}

/// Walks `dir` (under `root`), recording each git repository it finds by its
/// store-relative path and pruning the descent there — a repository is never
/// searched for repositories inside it, matching gitweb's `File::Find` prune.
fn scan(root: &Path, dir: &Path, found: &mut Vec<Project>) -> Result<(), DomainError> {
    let entries: fs::ReadDir = fs::read_dir(dir).map_err(backend)?;
    for entry in entries {
        let path: PathBuf = entry.map_err(backend)?.path();
        // Only directories are candidates; `is_dir` follows symlinks, as gitweb
        // does with `follow_fast`.
        if !path.is_dir() {
            continue;
        }
        if head_linked(&path) {
            found.push(Project::new(relative_name(root, &path)));
        } else {
            scan(root, &path, found)?;
        }
    }
    Ok(())
}

/// gitweb's `check_head_link`: `HEAD` exists (a file, or a symlink to an
/// existing target), or is a symlink whose target is under `refs/heads/`.
fn head_linked(dir: &Path) -> bool {
    let head: PathBuf = dir.join("HEAD");
    if head.exists() {
        return true;
    }
    // A dangling symlink still counts when it points into refs/heads/.
    match fs::symlink_metadata(&head) {
        Ok(meta) if meta.file_type().is_symlink() => fs::read_link(&head)
            .map(|target: PathBuf| target.starts_with("refs/heads"))
            .unwrap_or(false),
        _ => false,
    }
}

/// The store-relative name of a repository directory: its path under `root`,
/// always joined with `/` so a name is portable and matches gitweb's paths.
fn relative_name(root: &Path, path: &Path) -> String {
    let relative: &Path = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component: std::path::Component<'_>| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
