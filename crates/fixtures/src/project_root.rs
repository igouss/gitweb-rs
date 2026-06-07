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
use crate::spec::{CommitSpec, Identity, Mode, TreeEntry};

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
}

impl Default for ProjectRoot {
    fn default() -> Self {
        Self::new()
    }
}
