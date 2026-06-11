//! The `tree` use case: gitweb's `git_tree` / `git_print_tree_entry`, minus the
//! HTML.
//!
//! gitweb lists one directory of a revision. The base revision (gitweb's
//! `$hash_base`, defaulting to `HEAD`) is peeled to a commit for the page header
//! and to its tree; an optional path descends into a subtree of it
//! (`git_get_hash_by_path`). Each entry becomes a row carrying its mode — which
//! already names the permission string and the git object type
//! ([`FileMode::object_kind`]) the links branch on — its name, the object it
//! points at, and, when `show-sizes` is on, the `git ls-tree -l` byte size (read
//! from the header, never by inflating the blob). A symlink additionally carries
//! the path it points to, read from its (tiny) blob.
//!
//! Two facts shared by the whole view live on it, not on a row: the commit
//! subject for the header (absent when the base names a raw tree, gitweb's
//! `else` branch that just prints the hash) and the parent directory the `..`
//! row links to (present only when browsing inside a directory).
//!
//! Sizes are the only feature-gated bit: `show-sizes` defaults on, so a row's
//! size is `Some` for a file and `None` (rendered `-`) for a directory or
//! gitlink; with the feature off the use case reads no sizes at all.

use crate::error::DomainError;
use crate::model::file_mode::{FileKind, FileMode};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::tree::TreeEntry;
use crate::port::repository::Repository;

/// One listed tree entry: its mode (carrying the permission string and the git
/// object type), leaf name, the object it points at, the optional `ls-tree -l`
/// size, and — for a symlink — the path it points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    mode: FileMode,
    name: String,
    oid: String,
    size: Option<u64>,
    symlink_target: Option<String>,
}

impl TreeRow {
    /// The entry's mode (its permission string and kind).
    #[must_use]
    pub fn mode(&self) -> FileMode {
        self.mode
    }

    /// The git object type this entry names — [`ObjectKind::Tree`] for a
    /// directory, [`ObjectKind::Commit`] for a gitlink, [`ObjectKind::Blob`] for
    /// a file — the value gitweb's `git_print_tree_entry` branches on.
    #[must_use]
    pub fn object_kind(&self) -> ObjectKind {
        self.mode.object_kind()
    }

    /// The entry's leaf name within the listed directory.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The id of the object this entry points at (full hex).
    #[must_use]
    pub fn oid(&self) -> &str {
        &self.oid
    }

    /// The entry's byte size when `show-sizes` is on and the entry is a file
    /// (gitweb's `ls-tree -l` size); `None` for a directory or gitlink (gitweb's
    /// `-`), and `None` for every entry when the feature is off.
    #[must_use]
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// For a symlink, the path it points to (gitweb's `git_get_link_target`);
    /// `None` for every other kind.
    #[must_use]
    pub fn symlink_target(&self) -> Option<&str> {
        self.symlink_target.as_deref()
    }
}

/// The `..` parent-directory affordance: present only when browsing inside a
/// directory. Its `path` is the directory to go up to — `None` when that is the
/// repository root (gitweb's `undef $up`), so the link drops the path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeParent {
    path: Option<String>,
}

impl TreeParent {
    /// The parent directory to link `..` to, or `None` for the repository root.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

/// The assembled directory listing: the resolved base id (for the raw-tree
/// header and as the link context), the commit subject for the header, the
/// browsed path, the `..` affordance, whether sizes are shown, and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeView {
    base: String,
    commit_title: Option<String>,
    path: Option<String>,
    parent: Option<TreeParent>,
    show_sizes: bool,
    rows: Vec<TreeRow>,
}

impl TreeView {
    /// The resolved id of the base the listing is rooted at (gitweb's
    /// `$hash_base`). When no commit subject is shown, this is the title.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The commit subject shown in the header, or `None` when the base names a
    /// raw tree rather than a commit (gitweb prints the hash instead).
    #[must_use]
    pub fn commit_title(&self) -> Option<&str> {
        self.commit_title.as_deref()
    }

    /// The browsed directory path (gitweb's `$file_name`), or `None` at the
    /// repository root.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The `..` parent affordance, present only when browsing inside a directory.
    #[must_use]
    pub fn parent(&self) -> Option<&TreeParent> {
        self.parent.as_ref()
    }

    /// Whether the size column is shown (gitweb's `show-sizes` feature).
    #[must_use]
    pub fn show_sizes(&self) -> bool {
        self.show_sizes
    }

    /// The listed entries, in the order git returned them.
    #[must_use]
    pub fn rows(&self) -> &[TreeRow] {
        &self.rows
    }
}

/// Assembles the directory listing over the [`Repository`] port. `base_rev` is
/// the revision to root at (`None` for `HEAD`, gitweb's `$hash_base`); `path` is
/// an optional subdirectory to descend into; `show_sizes` is gitweb's
/// `show-sizes` feature.
///
/// # Errors
///
/// Propagates the repository's error if the base does not resolve or its objects
/// cannot be read. Returns [`DomainError::NotFound`] — gitweb's
/// `die_error(404, "No such tree")` — when `path` names nothing, or names
/// something that is not a directory, in the base's tree. Returns
/// [`DomainError::Invalid`] when the base resolves to neither a commit nor a
/// tree (it is not a tree-ish to list).
pub fn assemble_tree(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    path: Option<&str>,
    show_sizes: bool,
) -> Result<TreeView, DomainError> {
    let base: ObjectId = repo.resolve(base_rev.unwrap_or("HEAD"))?;
    // The commit subject for the header and the tree to root the listing at:
    // a commit is peeled to its tree (gitweb's `ls-tree` on a commit-ish), a
    // raw tree is listed directly, and anything else is not a tree-ish.
    let (commit_title, root_tree): (Option<String>, ObjectId) = match repo.object_kind(&base)? {
        ObjectKind::Commit => {
            let commit = repo.find_commit(&base)?;
            (Some(commit.title()), commit.tree().clone())
        }
        ObjectKind::Tree => (None, base.clone()),
        other => {
            return Err(DomainError::Invalid(format!(
                "not a tree-ish: {} is a {}",
                base.as_str(),
                other.as_str()
            )));
        }
    };
    let listed: ObjectId = descend(repo, &base, root_tree, path)?;
    let tree = repo.find_tree(&listed)?;
    let rows: Vec<TreeRow> = tree
        .entries()
        .iter()
        .map(|entry: &TreeEntry| row_of(repo, entry, show_sizes))
        .collect::<Result<Vec<TreeRow>, DomainError>>()?;
    Ok(TreeView {
        base: base.as_str().to_owned(),
        commit_title,
        path: path.map(str::to_owned),
        parent: parent_of(path),
        show_sizes,
        rows,
    })
}

/// The tree to list: the base's root tree at the repository root, or the subtree
/// `path` names within it (gitweb's `git_get_hash_by_path($hash_base, $path,
/// "tree")`). A path that names nothing, or names something that is not a tree,
/// is `die_error(404, "No such tree")`.
///
/// The type check is on the entry's mode — the column `git ls-tree` derives from
/// the mode, which is exactly what `git_get_hash_by_path` compares against its
/// `"tree"` filter (gitweb.perl:2899). A gitlink reads as a commit straight from
/// its mode, so the listing 404s without ever reaching for the submodule's commit
/// object (which does not live in this repository).
fn descend(
    repo: &dyn Repository,
    base: &ObjectId,
    root_tree: ObjectId,
    path: Option<&str>,
) -> Result<ObjectId, DomainError> {
    let path: &str = match path {
        Some(path) if !path.is_empty() => path,
        _ => return Ok(root_tree),
    };
    let entry: TreeEntry = repo
        .path_entry(base, path)?
        .ok_or_else(|| DomainError::NotFound("No such tree".to_owned()))?;
    if entry.mode().object_kind() != ObjectKind::Tree {
        return Err(DomainError::NotFound("No such tree".to_owned()));
    }
    Ok(entry.oid().clone())
}

/// Maps one tree entry to a row: its mode, name, and object id, plus the size and
/// symlink target [`size_and_target`] resolves.
fn row_of(
    repo: &dyn Repository,
    entry: &TreeEntry,
    show_sizes: bool,
) -> Result<TreeRow, DomainError> {
    let (size, symlink_target): (Option<u64>, Option<String>) =
        size_and_target(repo, entry, show_sizes)?;
    Ok(TreeRow {
        mode: entry.mode(),
        name: entry.name().to_owned(),
        oid: entry.oid().as_str().to_owned(),
        size,
        symlink_target,
    })
}

/// The parent affordance for the browsed `path`: present only inside a
/// directory, linking to the directory above (gitweb's `$up`, `None` at the
/// repository root).
fn parent_of(path: Option<&str>) -> Option<TreeParent> {
    match path {
        Some(path) if !path.is_empty() => Some(TreeParent {
            path: parent_dir(path),
        }),
        _ => None,
    }
}

/// The byte size and symlink target for one entry: a file gets its size (when
/// `show_sizes`); a symlink also gets its target, read from its blob (whose
/// content *is* the target, so the size comes from the same read); a directory
/// or gitlink gets neither.
fn size_and_target(
    repo: &dyn Repository,
    entry: &TreeEntry,
    show_sizes: bool,
) -> Result<(Option<u64>, Option<String>), DomainError> {
    let oid: &ObjectId = entry.oid();
    if entry.mode().kind() == FileKind::Symlink {
        let blob = repo.find_blob(oid)?;
        let target: String = String::from_utf8_lossy(blob.bytes()).into_owned();
        let size: Option<u64> = show_sizes.then(|| blob.size() as u64);
        return Ok((size, Some(target)));
    }
    if entry.mode().object_kind() == ObjectKind::Blob {
        let size: Option<u64> = if show_sizes {
            Some(repo.object_size(oid)?)
        } else {
            None
        };
        return Ok((size, None));
    }
    Ok((None, None))
}

/// gitweb's `$up =~ s!/?[^/]+$!!`: the directory above `path`, or `None` when
/// that is the repository root.
fn parent_dir(path: &str) -> Option<String> {
    let trimmed: &str = path.trim_end_matches('/');
    trimmed
        .rfind('/')
        .map(|index: usize| trimmed[..index].to_owned())
}
