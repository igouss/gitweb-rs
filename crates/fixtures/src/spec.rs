//! Plain input data for the [`crate::RepoBuilder`].
//!
//! These types say *what* object to write, never *how*. They are deliberately
//! gix-free apart from re-exported [`ObjectId`], so specs that build fixtures
//! read like a description of the repository, not like git plumbing.

use gix::ObjectId;

/// A git file mode, named by what it is rather than by its octal bits.
///
/// These are the only modes git records in a tree:
/// `File` is `100644`, `Executable` `100755`, `Symlink` `120000`,
/// `Directory` `040000`, and `Gitlink` (a submodule commit) `160000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A regular, non-executable file (`100644`).
    File,
    /// An executable file (`100755`).
    Executable,
    /// A symbolic link, whose blob holds the link target (`120000`).
    Symlink,
    /// A subtree (`040000`).
    Directory,
    /// A submodule: a commit id recorded in the tree (`160000`).
    Gitlink,
}

/// A pinned author / committer / tagger identity.
///
/// Pinning the name, the email, *and* the timestamp is what makes the resulting
/// object ids stable across runs and machines — the whole point of a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The person's name, written verbatim into the object.
    pub name: String,
    /// The person's email, written verbatim into the object.
    pub email: String,
    /// Seconds since the Unix epoch.
    pub epoch_seconds: i64,
    /// Timezone offset from UTC, in seconds (e.g. `0` for `+0000`).
    pub timezone_offset_seconds: i32,
}

/// One entry in a tree: a name, what kind of thing it is, and the object it
/// names. Entries may be supplied in any order; the builder sorts them into
/// git's canonical tree order before writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// The entry's file name within its tree.
    pub name: String,
    /// What kind of object the entry names, and thus its mode.
    pub mode: Mode,
    /// The object the entry points at.
    pub oid: ObjectId,
}

/// Everything needed to write one commit deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSpec {
    /// The tree the commit snapshots.
    pub tree: ObjectId,
    /// Parent commits, in order; empty for a root commit, two or more for a
    /// merge.
    pub parents: Vec<ObjectId>,
    /// Who wrote the change.
    pub author: Identity,
    /// Who committed it.
    pub committer: Identity,
    /// The commit message, written verbatim — no trailing newline is added.
    pub message: String,
}

/// What kind of object an annotated tag points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// The tag names a commit (the common case).
    Commit,
    /// The tag names a tree.
    Tree,
    /// The tag names a blob.
    Blob,
    /// The tag names another tag.
    Tag,
}

/// Everything needed to write one annotated tag object deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSpec {
    /// The tag's own name (also the `refs/tags/` ref the builder creates).
    pub name: String,
    /// The object the tag points at.
    pub target: ObjectId,
    /// What kind of object the target is.
    pub target_kind: TargetKind,
    /// Who created the tag.
    pub tagger: Identity,
    /// The tag message, written verbatim.
    pub message: String,
}
