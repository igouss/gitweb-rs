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

    /// Lays down a bare repository at `name` whose single file `file.txt` is
    /// created and then modified across two commits on branch `main`, so the
    /// per-path history of `file.txt` has two rows: the newer "edit file.txt"
    /// (the current version) and the older "add file.txt" (whose differing blob
    /// earns a "diff to current"). Both commits sit before the two-week window, so
    /// their date cells show absolute dates (2023-11-14 / 2023-11-15).
    pub fn add_file_history(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);

        let added: Identity = Identity {
            epoch_seconds: 1_700_000_000,
            ..ada()
        };
        let edited: Identity = Identity {
            epoch_seconds: 1_700_086_400,
            ..ada()
        };

        let first_tree: gix::ObjectId = builder.tree(&[TreeEntry {
            name: "file.txt".to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"v1\n"),
        }]);
        let first: gix::ObjectId = builder.commit(&CommitSpec {
            tree: first_tree,
            parents: Vec::new(),
            author: added.clone(),
            committer: added,
            message: "add file.txt\n".to_owned(),
        });

        let second_tree: gix::ObjectId = builder.tree(&[TreeEntry {
            name: "file.txt".to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"v2\n"),
        }]);
        let second: gix::ObjectId = builder.commit(&CommitSpec {
            tree: second_tree,
            parents: vec![first],
            author: edited.clone(),
            committer: edited,
            message: "edit file.txt\n".to_owned(),
        });

        builder.branch("main", second);
        builder.set_head("main");
    }

    /// Lays down a bare repository at `name` whose `HEAD` commit exercises every
    /// ordinary changed-file kind against its single parent: it modifies `README`,
    /// deletes `gone.txt`, renames `old.txt` to `new.txt` (the blob is reused, so
    /// rename detection reports full similarity), and adds `NEWFILE`. Its message
    /// has a subject line and a body, so the commit view shows both.
    pub fn add_commit_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        let root_who: Identity = Identity {
            epoch_seconds: 1_700_000_000,
            ..ada()
        };
        let head_who: Identity = Identity {
            epoch_seconds: 1_700_100_000,
            ..ada()
        };

        let moved: gix::ObjectId = builder.blob(b"moved content\n");
        let root_tree: gix::ObjectId = builder.tree(&[
            TreeEntry {
                name: "README".to_owned(),
                mode: Mode::File,
                oid: builder.blob(b"first\n"),
            },
            TreeEntry {
                name: "gone.txt".to_owned(),
                mode: Mode::File,
                oid: builder.blob(b"bye\n"),
            },
            TreeEntry {
                name: "old.txt".to_owned(),
                mode: Mode::File,
                oid: moved,
            },
        ]);
        let root: gix::ObjectId = builder.commit(&CommitSpec {
            tree: root_tree,
            parents: Vec::new(),
            author: root_who.clone(),
            committer: root_who,
            message: "Initial import\n".to_owned(),
        });

        let head_tree: gix::ObjectId = builder.tree(&[
            TreeEntry {
                name: "README".to_owned(),
                mode: Mode::File,
                oid: builder.blob(b"second\n"),
            },
            TreeEntry {
                name: "new.txt".to_owned(),
                mode: Mode::File,
                oid: moved,
            },
            TreeEntry {
                name: "NEWFILE".to_owned(),
                mode: Mode::File,
                oid: builder.blob(b"brand new\n"),
            },
        ]);
        let head: gix::ObjectId = builder.commit(&CommitSpec {
            tree: head_tree,
            parents: vec![root],
            author: head_who.clone(),
            committer: head_who,
            message: "Rework the engine\n\nDetailed body line.\n".to_owned(),
        });

        builder.branch("main", head);
        builder.set_head("main");
    }

    /// Lays down a bare repository at `name` whose `HEAD` is a merge of two
    /// branches that each changed `file.txt`, resolved to a third value — so the
    /// combined diff against both parents lists `file.txt`.
    pub fn add_merge_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        let who: Identity = Identity {
            epoch_seconds: 1_700_200_000,
            ..ada()
        };
        let one_file = |content: &[u8]| -> gix::ObjectId {
            builder.tree(&[TreeEntry {
                name: "file.txt".to_owned(),
                mode: Mode::File,
                oid: builder.blob(content),
            }])
        };

        let base: gix::ObjectId = builder.commit(&CommitSpec {
            tree: one_file(b"base\n"),
            parents: Vec::new(),
            author: who.clone(),
            committer: who.clone(),
            message: "base\n".to_owned(),
        });
        let side_a: gix::ObjectId = builder.commit(&CommitSpec {
            tree: one_file(b"side A\n"),
            parents: vec![base],
            author: who.clone(),
            committer: who.clone(),
            message: "side a\n".to_owned(),
        });
        let side_b: gix::ObjectId = builder.commit(&CommitSpec {
            tree: one_file(b"side B\n"),
            parents: vec![base],
            author: who.clone(),
            committer: who.clone(),
            message: "side b\n".to_owned(),
        });
        let merge: gix::ObjectId = builder.commit(&CommitSpec {
            tree: one_file(b"merged\n"),
            parents: vec![side_a, side_b],
            author: who.clone(),
            committer: who,
            message: "Merge side branches\n".to_owned(),
        });

        builder.branch("main", merge);
        builder.set_head("main");
    }

    /// Lays down a bare repository at `name` whose `HEAD` commit holds one tree
    /// exercising every entry kind the tree view distinguishes: a regular file
    /// `README` (12 bytes), an executable `build.sh`, a symlink `latest` pointing
    /// to `src/main.rs`, a subdirectory `src` containing `main.rs`, and a
    /// gitlink/submodule `vendor`. The subdirectory lets a spec browse one level
    /// down and exercise the `..` parent row.
    pub fn add_tree_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        let who: Identity = ada();

        let readme: gix::ObjectId = builder.blob(b"hello world\n");
        let build: gix::ObjectId = builder.blob(b"#!/bin/sh\necho build\n");
        let link: gix::ObjectId = builder.blob(b"src/main.rs");
        let main_rs: gix::ObjectId = builder.blob(b"fn main() {}\n");
        let src_tree: gix::ObjectId = builder.tree(&[TreeEntry {
            name: "main.rs".to_owned(),
            mode: Mode::File,
            oid: main_rs,
        }]);
        // A gitlink points at a commit that need not exist in this repository.
        let submodule: gix::ObjectId =
            gix::ObjectId::from_hex(b"1111111111111111111111111111111111111111")
                .expect("a valid fixture gitlink id");

        let root_tree: gix::ObjectId = builder.tree(&[
            TreeEntry {
                name: "README".to_owned(),
                mode: Mode::File,
                oid: readme,
            },
            TreeEntry {
                name: "build.sh".to_owned(),
                mode: Mode::Executable,
                oid: build,
            },
            TreeEntry {
                name: "latest".to_owned(),
                mode: Mode::Symlink,
                oid: link,
            },
            TreeEntry {
                name: "src".to_owned(),
                mode: Mode::Directory,
                oid: src_tree,
            },
            TreeEntry {
                name: "vendor".to_owned(),
                mode: Mode::Gitlink,
                oid: submodule,
            },
        ]);
        let commit: gix::ObjectId = builder.commit(&CommitSpec {
            tree: root_tree,
            parents: Vec::new(),
            author: who.clone(),
            committer: who,
            message: "tree fixture\n".to_owned(),
        });
        builder.branch("main", commit);
        builder.set_head("main");
    }

    /// Lays down a bare repository at `name` whose `HEAD` commit holds one tree
    /// of blobs exercising every display kind the blob view distinguishes: a
    /// UTF-8 text file `readme.txt`, a non-UTF-8 (latin1) text file `latin1.txt`,
    /// an empty file `empty.txt`, a NUL-bearing binary `data.bin`, and a
    /// NUL-bearing `logo.png` whose extension makes it an inline image.
    pub fn add_blob_repo(&self, name: &str) {
        let full: PathBuf = self.dir.path().join(name);
        let builder: RepoBuilder = RepoBuilder::init_at(&full);
        let who: Identity = ada();

        let readme: gix::ObjectId = builder.blob(b"hello world\nsecond line\n");
        // "Hello" with an e-acute (0xe9) — valid latin1, invalid UTF-8, no NUL.
        let latin1: gix::ObjectId = builder.blob(&[0x48, 0xe9, 0x6c, 0x6c, 0x6f]);
        let empty: gix::ObjectId = builder.blob(b"");
        let data: gix::ObjectId = builder.blob(&[0x00, 0x01, 0x02, 0x03, 0xff]);
        // A PNG signature: it carries a NUL (binary) and its name an image
        // extension, so the blob view shows it inline as an image.
        let logo: gix::ObjectId =
            builder.blob(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00]);

        let root_tree: gix::ObjectId = builder.tree(&[
            TreeEntry {
                name: "readme.txt".to_owned(),
                mode: Mode::File,
                oid: readme,
            },
            TreeEntry {
                name: "latin1.txt".to_owned(),
                mode: Mode::File,
                oid: latin1,
            },
            TreeEntry {
                name: "empty.txt".to_owned(),
                mode: Mode::File,
                oid: empty,
            },
            TreeEntry {
                name: "data.bin".to_owned(),
                mode: Mode::File,
                oid: data,
            },
            TreeEntry {
                name: "logo.png".to_owned(),
                mode: Mode::File,
                oid: logo,
            },
        ]);
        let commit: gix::ObjectId = builder.commit(&CommitSpec {
            tree: root_tree,
            parents: Vec::new(),
            author: who.clone(),
            committer: who,
            message: "blob fixture\n".to_owned(),
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

    /// Points an arbitrary fully-qualified ref `full_ref` (e.g.
    /// `refs/sandbox/wip`) at a fresh commit committed at `epoch` in the repository
    /// at `name`. This builds the refs outside `refs/heads/` and `refs/remotes/`
    /// that the `extra-branch-refs` feature treats as branches — used to verify a
    /// snapshot of such a ref is named with its directory prefix.
    pub fn add_ref_at(&self, name: &str, full_ref: &str, epoch: i64) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let commit: gix::ObjectId = commit_at(&builder, full_ref, epoch);
        builder.custom_ref(full_ref, commit);
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

    /// Points a lightweight `refs/tags/<tag>` straight at a blob in the
    /// repository at `name`. A tag of a non-commit object carries no creator
    /// date, so gitweb's tag listing shows it with no age cell at all (not
    /// "unknown") — the empty-age case the listing must distinguish.
    pub fn add_lightweight_blob_tag(&self, name: &str, tag: &str) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let blob: gix::ObjectId = builder.blob(tag.as_bytes());
        builder.lightweight_tag(tag, blob);
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

    /// Configures a remote (`[remote "<remote>"]`) in the repository at `name`,
    /// with the given fetch (`url`) and push (`pushurl`) URLs — either omitted to
    /// build the fetch-only / push-only / no-URL shapes. This is what the gix
    /// adapter's `remotes()` reads for the remotes view.
    pub fn add_remote(
        &self,
        name: &str,
        remote: &str,
        fetch_url: Option<&str>,
        push_url: Option<&str>,
    ) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        builder.remote(remote, fetch_url, push_url);
    }

    /// Points `refs/remotes/<remote>/<branch>` at a fresh commit committed at
    /// `epoch` in the repository at `name` — one remote-tracking branch for the
    /// remotes view to list under its remote.
    pub fn add_remote_branch(&self, name: &str, remote: &str, branch: &str, epoch: i64) {
        let builder: RepoBuilder = RepoBuilder::open_at(&self.dir.path().join(name));
        let commit: gix::ObjectId = commit_at(&builder, &format!("{remote}/{branch}"), epoch);
        builder.remote_branch(remote, branch, commit);
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

    /// Writes the repository's `README.html`, the raw-HTML file gitweb injects
    /// verbatim into the summary page. Written under the repository's git
    /// directory exactly as gitweb reads it (`$GIT_DIR/README.html`); the bytes
    /// are stored as given, so a spec can write a non-empty or an empty file to
    /// exercise gitweb's `-s` test.
    pub fn set_readme_html(&self, name: &str, contents: &str) {
        self.write_metadata_file(name, "README.html", contents);
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
