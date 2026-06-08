//! The deterministic fixture repository the goldens are captured over.
//!
//! [`build`] writes one bare repository, `repo.git`, under a caller-owned
//! project root. It is the single source of truth for the corpus: the
//! `build-corpus` binary runs it to lay the repository down for `gitweb.perl` to
//! read at capture time, and the golden test runs it to rebuild the very same
//! repository for the gix adapter to read at test time. Because the builder is
//! deterministic, both constructions produce identical object ids, so a golden
//! captured for a blob corresponds exactly to the blob the test reads back.

use std::fs;
use std::path::{Path, PathBuf};

use gitweb_fixtures::{
    CommitSpec, Identity, Mode, ObjectId, RepoBuilder, TagSpec, TargetKind, TreeEntry,
};

/// One named blob in the corpus: its stable logical name (how features and
/// goldens refer to it), its object id, and the path it lives at in the tree (how
/// `gitweb.perl` addresses it, `f=<file_name>;hb=HEAD`).
#[derive(Debug, Clone)]
pub struct BlobFixture {
    pub name: String,
    pub oid: ObjectId,
    pub file_name: String,
}

/// The built corpus: where the repository sits and every named blob in it.
#[derive(Debug)]
pub struct Corpus {
    /// The project root `gitweb.perl` scans; the repository is `repo.git` under it.
    pub project_root: PathBuf,
    /// The on-disk path of the bare repository (`<project_root>/repo.git`).
    pub repo_path: PathBuf,
    /// Each named blob, in capture order.
    pub blobs: Vec<BlobFixture>,
    /// The parent commit of the two-commit `diffs` branch — the `blobdiff_plain`
    /// goldens' `hpb`. It lives off a branch that is *not* `HEAD`, so the
    /// single-commit goldens (blob_plain, feed, commitdiff_plain) are untouched.
    pub diff_parent: ObjectId,
    /// The head commit of the `diffs` branch — the `blobdiff_plain` goldens' `hb`.
    pub diff_head: ObjectId,
}

impl Corpus {
    /// The git project name `gitweb.perl` addresses this repository by (`p=`),
    /// relative to the project root.
    pub const PROJECT: &'static str = "repo.git";

    /// The pinned project owner (`gitweb.owner`). Feeds put the owner in their
    /// body (`<managingEditor>` / feed author); the filesystem-owner fallback
    /// would otherwise yield the capturing user's name, making the golden
    /// non-reproducible. The golden test reads this back through the adapter.
    pub const OWNER: &'static str = "Ada Lovelace";

    /// The pinned project description (the `description` file's first line) —
    /// the feed's `<description>` / `<subtitle>` — for the same reason.
    pub const DESCRIPTION: &'static str = "Deterministic gitweb-rs parity corpus.";

    /// The `diffs` branch's file paths, shared by the corpus builder, the
    /// `blobdiff_plain` golden test, and the capture script's query strings.
    /// The text file modified between the two commits.
    pub const DIFF_TEXT: &'static str = "a.txt";
    /// The file whose mode changes (100644 → 100755) with no content change.
    pub const DIFF_MODE: &'static str = "mode.sh";
    /// The binary file modified between the two commits.
    pub const DIFF_BINARY: &'static str = "bin.dat";
    /// The rename's from-path (the `fp=` of the rename golden).
    pub const DIFF_RENAME_FROM: &'static str = "old.txt";
    /// The rename's to-path (the `f=` of the rename golden).
    pub const DIFF_RENAME_TO: &'static str = "new.txt";

    /// The object id of the blob named `name`.
    ///
    /// # Panics
    /// Panics if no blob carries that name — a corpus and a feature that disagree
    /// on a name is a broken test, and should fail loudly.
    #[must_use]
    pub fn blob(&self, name: &str) -> ObjectId {
        self.blobs
            .iter()
            .find(|fixture: &&BlobFixture| fixture.name == name)
            .map(|fixture: &BlobFixture| fixture.oid)
            .unwrap_or_else(|| panic!("no corpus blob named {name}"))
    }

    /// The tree path of the blob named `name` — how `gitweb.perl` addressed it at
    /// capture time (`f=<file_name>`), and the name our headers are derived from.
    ///
    /// # Panics
    /// Panics if no blob carries that name (see [`Corpus::blob`]).
    #[must_use]
    pub fn file_name(&self, name: &str) -> &str {
        self.blobs
            .iter()
            .find(|fixture: &&BlobFixture| fixture.name == name)
            .map(|fixture: &BlobFixture| fixture.file_name.as_str())
            .unwrap_or_else(|| panic!("no corpus blob named {name}"))
    }
}

/// One blob to seed: its logical name, the tree path it lives at, and its bytes.
struct Seed {
    name: &'static str,
    file_name: &'static str,
    content: &'static [u8],
}

/// The blobs span what a byte-stable blob endpoint has to survive: an empty blob
/// (zero bytes), a one-byte blob, ordinary UTF-8 text, binary content with a NUL,
/// and Latin-1 content with a high byte that is not valid UTF-8.
const SEEDS: &[Seed] = &[
    Seed {
        name: "empty",
        file_name: "empty.txt",
        content: b"",
    },
    Seed {
        name: "one",
        file_name: "one.txt",
        content: b"x",
    },
    Seed {
        name: "text",
        file_name: "hello.txt",
        content: b"hello gitweb\n",
    },
    Seed {
        name: "binary",
        file_name: "binary.bin",
        content: &[0x00, 0x01, 0x02, 0xFF],
    },
    Seed {
        name: "latin1",
        file_name: "latin1.txt",
        content: b"caf\xE9\n",
    },
];

/// The two-commit `diffs` branch the `blobdiff_plain` goldens are captured over,
/// spanning the single-file diff surface that endpoint must reproduce: a text
/// modification (hunk + abbreviated index), a pure file-mode change (no index, no
/// hunk), a binary modification (the `Binary files … differ` notice — bare
/// `diff-tree -p` emits no base85 patch), and an exact rename (`similarity index
/// 100%`, no hunk). It hangs off a branch other than `HEAD` so the single-commit
/// goldens stay byte-identical.
///
/// Returns the parent and head commit ids — the goldens' `hpb` and `hb`.
fn build_diffs_branch(builder: &RepoBuilder, who: &Identity) -> (ObjectId, ObjectId) {
    // The mode-change file shares one blob across both commits (only the tree
    // mode differs), and the rename's two paths share one blob (an exact rename).
    let mode_blob: ObjectId = builder.blob(b"#!/bin/sh\necho hi\n");
    let moved_blob: ObjectId = builder.blob(b"to be moved\n");

    let parent_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::DIFF_TEXT.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"alpha\nbeta\ngamma\n"),
        },
        TreeEntry {
            name: Corpus::DIFF_MODE.to_owned(),
            mode: Mode::File,
            oid: mode_blob,
        },
        TreeEntry {
            name: Corpus::DIFF_BINARY.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&[0x00, 0x01, 0x02, 0xFF]),
        },
        TreeEntry {
            name: Corpus::DIFF_RENAME_FROM.to_owned(),
            mode: Mode::File,
            oid: moved_blob,
        },
    ]);
    let parent: ObjectId = builder.commit(&CommitSpec {
        tree: parent_tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "diffs base\n".to_owned(),
    });

    let head_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::DIFF_TEXT.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"alpha\nBETA\ngamma\n"),
        },
        TreeEntry {
            name: Corpus::DIFF_MODE.to_owned(),
            mode: Mode::Executable,
            oid: mode_blob,
        },
        TreeEntry {
            name: Corpus::DIFF_BINARY.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&[0x00, 0x01, 0x02, 0x03, 0xFF]),
        },
        TreeEntry {
            name: Corpus::DIFF_RENAME_TO.to_owned(),
            mode: Mode::File,
            oid: moved_blob,
        },
    ]);
    let head: ObjectId = builder.commit(&CommitSpec {
        tree: head_tree,
        parents: vec![parent],
        author: who.clone(),
        committer: who.clone(),
        message: "diffs head\n".to_owned(),
    });

    builder.branch("diffs", head);
    (parent, head)
}

/// Pins the project's `gitweb.owner` config and `description` file, so the feed
/// metadata is reproducible across machines. Both gitweb (at capture) and the
/// gix adapter (at test time) read these back; written to the files directly,
/// the way `git config gitweb.owner` and the `description` file are read.
fn pin_metadata(repo_path: &Path) {
    let config_path: PathBuf = repo_path.join("config");
    let mut config: String = fs::read_to_string(&config_path).expect("read the corpus repo config");
    config.push_str(&format!("[gitweb]\n\towner = {}\n", Corpus::OWNER));
    fs::write(&config_path, config).expect("write the corpus repo config");
    fs::write(
        repo_path.join("description"),
        format!("{}\n", Corpus::DESCRIPTION),
    )
    .expect("write the corpus repo description");
}

/// The single pinned identity, matching the adapter conformance fixture so the
/// whole tree's history is reproducible to the byte.
fn ada() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_700_000_000,
        timezone_offset_seconds: 0,
    }
}

/// Builds the parity corpus under `project_root` (a caller-owned directory that
/// must not already hold `repo.git`). The blobs hang off a single root commit so
/// the repository has a real `HEAD` to resolve `hb=HEAD` against.
#[must_use]
pub fn build(project_root: &Path) -> Corpus {
    let repo_path: PathBuf = project_root.join(Corpus::PROJECT);
    let builder: RepoBuilder = RepoBuilder::init_at(&repo_path);
    let who: Identity = ada();

    let mut blobs: Vec<BlobFixture> = Vec::with_capacity(SEEDS.len());
    let mut entries: Vec<TreeEntry> = Vec::with_capacity(SEEDS.len());
    for seed in SEEDS {
        let oid: ObjectId = builder.blob(seed.content);
        entries.push(TreeEntry {
            name: seed.file_name.to_owned(),
            mode: Mode::File,
            oid,
        });
        blobs.push(BlobFixture {
            name: seed.name.to_owned(),
            oid,
            file_name: seed.file_name.to_owned(),
        });
    }

    let tree: ObjectId = builder.tree(&entries);
    let root: ObjectId = builder.commit(&CommitSpec {
        tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "corpus root\n".to_owned(),
    });
    builder.branch("main", root);
    builder.set_head("main");

    // An annotated tag whose tip is the root commit, so `commitdiff_plain` of
    // HEAD carries gitweb's `X-Git-Tag` line. `git name-rev --tags` renders an
    // annotated tag's name with its one-dereference marker, `v1.0^0`. A tag is a
    // ref pointing at an existing commit (plus a tag object that lives outside the
    // commit graph), so it leaves HEAD's id and every other golden untouched.
    let _tag: ObjectId = builder.annotated_tag(&TagSpec {
        name: "v1.0".to_owned(),
        target: root,
        target_kind: TargetKind::Commit,
        tagger: who.clone(),
        message: "release 1.0\n".to_owned(),
    });

    // The two-commit `diffs` branch the blobdiff_plain goldens diff across. It
    // is built after HEAD is set to `main`, so HEAD stays on the root commit and
    // the single-commit goldens are unaffected.
    let (diff_parent, diff_head): (ObjectId, ObjectId) = build_diffs_branch(&builder, &who);

    pin_metadata(&repo_path);

    Corpus {
        project_root: project_root.to_path_buf(),
        repo_path,
        blobs,
        diff_parent,
        diff_head,
    }
}
