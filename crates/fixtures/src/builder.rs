//! The fixture builder: writes git objects and refs with gix.
//!
//! Object ids are real git SHA-1s. Tree entries are sorted into git's canonical
//! order before writing — gix refuses to serialize an unsorted tree — and every
//! identity and timestamp comes straight from the caller, so nothing here is
//! non-deterministic.

use std::path::Path;

use gix::ObjectId;
use gix::objs::tree::{Entry, EntryKind};
use gix::objs::{Commit, Tag, Tree};
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target, TargetRef};

use crate::spec::{CommitSpec, Identity, Mode, TagSpec, TargetKind, TreeEntry};

/// A throwaway bare git repository that builds deterministic fixtures.
///
/// A builder from [`RepoBuilder::init`] owns its own temp directory, deleted
/// when the builder is dropped. A builder from [`RepoBuilder::init_at`] writes
/// into a caller-owned directory (e.g. a [`ProjectRoot`]) and deletes nothing on
/// drop — the owner handles cleanup. Methods panic on any internal git failure:
/// a fixture that cannot be built is a broken test, and should fail loudly and
/// immediately.
///
/// [`ProjectRoot`]: crate::ProjectRoot
#[derive(Debug)]
pub struct RepoBuilder {
    repo: gix::Repository,
    /// The temp dir to delete on drop, when this builder owns one.
    _dir: Option<tempfile::TempDir>,
}

fn entry_kind(mode: Mode) -> EntryKind {
    match mode {
        Mode::File => EntryKind::Blob,
        Mode::Executable => EntryKind::BlobExecutable,
        Mode::Symlink => EntryKind::Link,
        Mode::Directory => EntryKind::Tree,
        Mode::Gitlink => EntryKind::Commit,
    }
}

fn object_kind(kind: TargetKind) -> gix::object::Kind {
    match kind {
        TargetKind::Commit => gix::object::Kind::Commit,
        TargetKind::Tree => gix::object::Kind::Tree,
        TargetKind::Blob => gix::object::Kind::Blob,
        TargetKind::Tag => gix::object::Kind::Tag,
    }
}

fn signature(who: &Identity) -> gix::actor::Signature {
    gix::actor::Signature {
        name: who.name.as_str().into(),
        email: who.email.as_str().into(),
        time: gix::date::Time::new(who.epoch_seconds, who.timezone_offset_seconds),
    }
}

impl RepoBuilder {
    /// A fresh, empty bare repository in a throwaway temp directory. Its HEAD is
    /// unborn — it points at a branch that does not exist yet.
    #[must_use]
    pub fn init() -> Self {
        let dir: tempfile::TempDir = tempfile::tempdir().expect("a temp dir for the fixture repo");
        let repo: gix::Repository = gix::init_bare(dir.path()).expect("init a bare fixture repo");
        Self {
            repo,
            _dir: Some(dir),
        }
    }

    /// A fresh, empty bare repository at a caller-owned `path` (its parents are
    /// created as needed). Cleanup is the caller's: this builder deletes nothing
    /// on drop. Used to lay several repositories under one [`ProjectRoot`].
    ///
    /// [`ProjectRoot`]: crate::ProjectRoot
    #[must_use]
    pub fn init_at(path: &Path) -> Self {
        std::fs::create_dir_all(path).expect("create the fixture repo directory");
        let repo: gix::Repository = gix::init_bare(path).expect("init a bare fixture repo");
        Self { repo, _dir: None }
    }

    /// Reopens an existing bare repository at a caller-owned `path`, to add more
    /// objects or refs to a fixture built earlier. Like [`Self::init_at`], it
    /// owns no temp directory and deletes nothing on drop.
    #[must_use]
    pub fn open_at(path: &Path) -> Self {
        let repo: gix::Repository = gix::open(path).expect("open an existing fixture repo");
        Self { repo, _dir: None }
    }

    /// The on-disk path of the repository, for adapters that open it by path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Writes a blob holding exactly these bytes; returns its id.
    #[must_use]
    pub fn blob(&self, content: &[u8]) -> ObjectId {
        self.repo
            .write_blob(content)
            .expect("write a fixture blob")
            .detach()
    }

    /// Writes a tree; entries may be in any order. Returns its id.
    #[must_use]
    pub fn tree(&self, entries: &[TreeEntry]) -> ObjectId {
        let mut written: Vec<Entry> = entries
            .iter()
            .map(|entry: &TreeEntry| Entry {
                mode: entry_kind(entry.mode).into(),
                filename: entry.name.as_str().into(),
                oid: entry.oid,
            })
            .collect();
        // git requires canonical order; gix asserts it rather than sorting.
        written.sort();
        self.repo
            .write_object(&Tree { entries: written })
            .expect("write a fixture tree")
            .detach()
    }

    /// Writes a commit; returns its id.
    #[must_use]
    pub fn commit(&self, spec: &CommitSpec) -> ObjectId {
        let commit: Commit = Commit {
            tree: spec.tree,
            parents: spec.parents.iter().copied().collect(),
            author: signature(&spec.author),
            committer: signature(&spec.committer),
            encoding: None,
            message: spec.message.as_str().into(),
            extra_headers: Vec::new(),
        };
        self.repo
            .write_object(&commit)
            .expect("write a fixture commit")
            .detach()
    }

    /// Points `refs/heads/<name>` at `target`.
    pub fn branch(&self, name: &str, target: ObjectId) {
        self.repo
            .reference(
                format!("refs/heads/{name}"),
                target,
                PreviousValue::Any,
                "fixture branch",
            )
            .expect("create a fixture branch");
    }

    /// Points `HEAD` symbolically at `refs/heads/<branch>`. The branch need not
    /// exist yet, which leaves HEAD unborn.
    pub fn set_head(&self, branch: &str) {
        let target_name: FullName = format!("refs/heads/{branch}")
            .try_into()
            .expect("a valid branch ref name");
        let edit: RefEdit = RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "fixture set HEAD".into(),
                },
                expected: PreviousValue::Any,
                new: Target::Symbolic(target_name),
            },
            name: "HEAD".try_into().expect("HEAD is a valid full name"),
            deref: false,
        };
        self.repo
            .edit_reference(edit)
            .expect("point HEAD at a branch");
    }

    /// Configures a remote in the repository's git config (`[remote "<name>"]`),
    /// writing `url` for `fetch_url` and `pushurl` for `push_url` when given. This
    /// is what `git remote -v` reads and the gix adapter's `remotes()` reports;
    /// either URL may be omitted to build the fetch-only / push-only / no-URL
    /// shapes gitweb's remote block handles. Written to the config file directly so
    /// a freshly-opened adapter reads it back.
    pub fn remote(&self, name: &str, fetch_url: Option<&str>, push_url: Option<&str>) {
        let config: std::path::PathBuf = self.repo.path().join("config");
        let mut text: String =
            std::fs::read_to_string(&config).expect("read the fixture repo config");
        text.push_str(&format!("[remote \"{name}\"]\n"));
        if let Some(url) = fetch_url {
            text.push_str(&format!("\turl = {url}\n"));
        }
        if let Some(url) = push_url {
            text.push_str(&format!("\tpushurl = {url}\n"));
        }
        std::fs::write(&config, text).expect("write the fixture repo config");
    }

    /// Points `refs/remotes/<remote>/<name>` at `target` — one remote-tracking
    /// branch, the refs gitweb's `fill_remote_heads` lists under a remote.
    pub fn remote_branch(&self, remote: &str, name: &str, target: ObjectId) {
        self.repo
            .reference(
                format!("refs/remotes/{remote}/{name}"),
                target,
                PreviousValue::Any,
                "fixture remote branch",
            )
            .expect("create a fixture remote-tracking branch");
    }

    /// Points `refs/tags/<name>` straight at `target` (a lightweight tag).
    pub fn lightweight_tag(&self, name: &str, target: ObjectId) {
        self.repo
            .reference(
                format!("refs/tags/{name}"),
                target,
                PreviousValue::Any,
                "fixture tag",
            )
            .expect("create a lightweight fixture tag");
    }

    /// Writes an annotated tag object and points `refs/tags/<spec.name>` at it;
    /// returns the tag object's id.
    #[must_use]
    pub fn annotated_tag(&self, spec: &TagSpec) -> ObjectId {
        let tag: Tag = Tag {
            target: spec.target,
            target_kind: object_kind(spec.target_kind),
            name: spec.name.as_str().into(),
            tagger: Some(signature(&spec.tagger)),
            message: spec.message.as_str().into(),
            pgp_signature: None,
        };
        let oid: ObjectId = self
            .repo
            .write_object(&tag)
            .expect("write an annotated tag")
            .detach();
        self.repo
            .reference(
                format!("refs/tags/{}", spec.name),
                oid,
                PreviousValue::Any,
                "fixture tag",
            )
            .expect("point a tag ref at its tag object");
        oid
    }

    /// The id `full_ref` resolves to, or `None` if it does not exist or is
    /// unborn. Symbolic refs such as `HEAD` are followed; tag objects are *not*
    /// peeled, so an annotated tag ref resolves to the tag object itself.
    #[must_use]
    pub fn id_of(&self, full_ref: &str) -> Option<ObjectId> {
        let reference: gix::Reference<'_> = self.repo.find_reference(full_ref).ok()?;
        if let Some(id) = reference.try_id() {
            return Some(id.detach());
        }
        let TargetRef::Symbolic(name) = reference.target() else {
            return None;
        };
        let inner: gix::Reference<'_> = self.repo.find_reference(name.as_bstr()).ok()?;
        inner.try_id().map(|id| id.detach())
    }

    /// The bytes of the blob named by `oid`, for round-trip assertions.
    #[must_use]
    pub fn read_blob(&self, oid: ObjectId) -> Vec<u8> {
        self.repo
            .find_object(oid)
            .expect("find a fixture blob")
            .data
            .clone()
    }
}
