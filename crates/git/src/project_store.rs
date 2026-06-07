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
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::safety::SafePath;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;

use crate::conv::backend;
use crate::repository::GixRepository;

/// Discovery and opening of the repositories under one project root, backed by
/// the filesystem and gix.
#[derive(Debug)]
pub struct GixProjectStore {
    root: PathBuf,
}

impl GixProjectStore {
    /// A store rooted at `root` — the `$projectroot` to scan and open against.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
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
}

impl ProjectStore for GixProjectStore {
    fn list(&self) -> Result<Vec<Project>, DomainError> {
        let mut found: Vec<Project> = Vec::new();
        scan(&self.root, &self.root, &mut found)?;
        // gitweb leaves order to File::Find; sort for a deterministic listing.
        found.sort_by(|a: &Project, b: &Project| a.name().cmp(b.name()));
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
        if let Some(owner) = config.string("gitweb.owner").map(cow_to_string) {
            info = info.with_owner(owner);
        }
        if let Some(category) = file_or_config(&git_dir, "category", &config, "gitweb.category") {
            info = info.with_category(category);
        }
        for url in clone_urls(&git_dir, &config) {
            info = info.with_clone_url(url);
        }
        Ok(info)
    }
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
