//! A project root: several repositories laid out under one directory.
//!
//! gitweb discovers repositories by walking `$projectroot`; the ProjectStore
//! adapter does the same. To exercise that, a spec needs a real directory tree
//! holding several git repositories at various depths, plus the odd plain
//! directory that must be ignored. [`ProjectRoot`] owns that tree in one temp
//! directory (deleted on drop) and builds each repository with [`RepoBuilder`],
//! so every object id is as deterministic as the rest of the fixtures.

use std::path::{Path, PathBuf};

use crate::builder::RepoBuilder;
use crate::spec::{CommitSpec, Identity, Mode, TagSpec, TargetKind, TreeEntry};

/// A throwaway directory holding several fixture repositories.
///
/// The whole tree lives in one temp directory, deleted when the root is
/// dropped. Methods panic on any internal failure: a fixture that cannot be
/// built is a broken test.
#[derive(Debug)]
pub struct ProjectRoot {
    dir: tempfile::TempDir,
}

/// The single pinned identity stamped into every fixture repository, so object
/// ids stay stable across runs and machines.
fn ada() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_700_000_000,
        timezone_offset_seconds: 0,
    }
}

/// Writes a one-file root commit whose author and committer times are both
/// `epoch`, with content keyed off `label` so distinct labels (branch or tag
/// names) yield distinct commits. Returns the commit id, for the caller to point
/// a branch or tag at.
fn commit_at(builder: &RepoBuilder, label: &str, epoch: i64) -> gix::ObjectId {
    let who: Identity = Identity {
        epoch_seconds: epoch,
        ..ada()
    };
    let blob: gix::ObjectId = builder.blob(format!("{label}\n").as_bytes());
    let tree: gix::ObjectId = builder.tree(&[TreeEntry {
        name: "file.txt".to_owned(),
        mode: Mode::File,
        oid: blob,
    }]);
    builder.commit(&CommitSpec {
        tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who,
        message: format!("{label}\n"),
    })
}

impl ProjectRoot {
    /// A fresh, empty project root in a throwaway temp directory.
    #[must_use]
    pub fn new() -> Self {
        let dir: tempfile::TempDir = tempfile::tempdir().expect("a temp dir for the project root");
        Self { dir }
    }

    /// The on-disk path of the root, for the adapter to scan and open against.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Lays down a bare repository at the store-relative `name` (its parent
    /// directories are created as needed), populated with one commit on branch
    /// `main` so an adapter can open it and read a real reference. The repository
    /// has a `HEAD`, so discovery recognises it as a project.
    pub fn add_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        let who: Identity = ada();

        let blob: gix::ObjectId = builder.blob(b"hello\n");
        let tree: gix::ObjectId = builder.tree(&[TreeEntry {
            name: "file.txt".to_owned(),
            mode: Mode::File,
            oid: blob,
        }]);
        let commit: gix::ObjectId = builder.commit(&CommitSpec {
            tree,
            parents: Vec::new(),
            author: who.clone(),
            committer: who,
            message: "init\n".to_owned(),
        });
        builder.branch("main", commit);
        builder.set_head("main");
    }

    /// Creates a plain directory at the store-relative `name` with no git
    /// repository inside it, to be ignored by discovery.
    pub fn add_dir(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        std::fs::create_dir_all(full).expect("create a plain fixture directory");
    }

    /// Lays down a bare repository at `name` with a `HEAD` but no commits — an
    /// unborn repository. gitweb still discovers it (the `HEAD` is present), but
    /// it has no branches, so its last activity is absent. Branches and tags are
    /// added afterwards with [`Self::add_branch_at`] / [`Self::add_tag_at`] so a
    /// spec controls each ref's commit time exactly.
    pub fn add_empty_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        builder.set_head("main");
    }

    /// Points `refs/heads/<branch>` at a fresh commit authored and committed at
    /// `epoch` (Unix seconds) in the repository at `name`. The commit's content
    /// is keyed off the branch name, so each branch is a distinct commit and
    /// "most recent branch wins" is testable by varying only the times.
    pub fn add_branch_at(&self, name: &str, branch: &str, epoch: i64) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let commit: gix::ObjectId = commit_at(&builder, branch, epoch);
        builder.branch(branch, commit);
    }

    /// Points a lightweight `refs/tags/<tag>` at a fresh commit committed at
    /// `epoch` in the repository at `name`, on no branch. This exercises gitweb's
    /// rule that only branch refs count for last activity: a newer tag must not
    /// move it.
    pub fn add_tag_at(&self, name: &str, tag: &str, epoch: i64) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let commit: gix::ObjectId = commit_at(&builder, tag, epoch);
        builder.lightweight_tag(tag, commit);
    }

    /// Writes an annotated `refs/tags/<tag>` pointing at a fresh commit committed
    /// at `epoch` in the repository at `name`, with the tagger time set to the
    /// same `epoch` and the given annotation message. This is the annotated
    /// counterpart of [`Self::add_tag_at`]: the tag listing must peel it, show
    /// its subject, and offer a `tag` selflink.
    pub fn add_annotated_tag_at(&self, name: &str, tag: &str, epoch: i64, message: &str) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let commit: gix::ObjectId = commit_at(&builder, tag, epoch);
        let tagger: Identity = Identity {
            epoch_seconds: epoch,
            ..ada()
        };
        let _oid: gix::ObjectId = builder.annotated_tag(&TagSpec {
            name: tag.to_owned(),
            target: commit,
            target_kind: TargetKind::Commit,
            tagger,
            message: format!("{message}\n"),
        });
    }

    /// Writes the repository's `description` file (gitweb reads its first line),
    /// overwriting the default one `git init` lays down.
    pub fn set_description(&self, name: &str, text: &str) {
        self.write_metadata_file(name, "description", &format!("{text}\n"));
    }

    /// Deletes the repository's `description` file, so gitweb falls back to the
    /// `gitweb.description` config value.
    pub fn remove_description(&self, name: &str) {
        let path: PathBuf = self.dir.path().join(name).join("description");
        std::fs::remove_file(path).expect("remove the fixture description file");
    }

    /// Writes the repository's `category` file (gitweb reads its first line).
    pub fn set_category(&self, name: &str, text: &str) {
        self.write_metadata_file(name, "category", &format!("{text}\n"));
    }

    /// Writes the repository's `cloneurl` file — one clone URL per line, in the
    /// order gitweb lists them.
    pub fn set_clone_urls(&self, name: &str, urls: &[&str]) {
        let body: String = urls
            .iter()
            .map(|url: &&str| format!("{url}\n"))
            .collect::<String>();
        self.write_metadata_file(name, "cloneurl", &body);
    }

    /// Appends a single `gitweb.<key> = <value>` entry to the repository's git
    /// config. Calling it twice with the same key records a multi-valued setting
    /// (e.g. several `gitweb.url` clone URLs).
    pub fn set_gitweb_config(&self, name: &str, key: &str, value: &str) {
        let config: PathBuf = self.dir.path().join(name).join("config");
        let mut text: String =
            std::fs::read_to_string(&config).expect("read the fixture repo config");
        text.push_str(&format!("[gitweb]\n\t{key} = {value}\n"));
        std::fs::write(&config, text).expect("write the fixture repo config");
    }

    /// Writes a `$projects_list` file (one entry per line) alongside the root and
    /// returns its path, for a store that lists from a file rather than scanning.
    pub fn write_projects_list(&self, body: &str) -> PathBuf {
        let path: PathBuf = self.dir.path().join("projects.list");
        std::fs::write(&path, body).expect("write the fixture projects-list file");
        path
    }

    /// Writes one metadata file at the root of the named repository.
    fn write_metadata_file(&self, name: &str, file: &str, contents: &str) {
        let path: PathBuf = self.dir.path().join(name).join(file);
        std::fs::write(path, contents).expect("write a fixture metadata file");
    }
}

impl Default for ProjectRoot {
    fn default() -> Self {
        Self::new()
    }
}
