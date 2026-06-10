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
    /// The text-only root commit of the `texts` branch — the single-commit `patch`
    /// golden's `h`. `git format-patch`'s binary patch body is git's own zlib
    /// output, which the gix-only, no-unsafe port cannot reproduce byte-exact, so
    /// the format-patch goldens are captured over commits with no binary file. Off
    /// a branch other than `HEAD`, so every other golden is byte-identical.
    pub text_commit: ObjectId,
    /// The head (tip) commit of the `texts` branch — the range `patches` golden's
    /// `h`. A second text-only commit on top of [`Corpus::text_commit`], so
    /// `git format-patch --root <tip>` emits the two-mail `[PATCH 1/2]` /
    /// `[PATCH 2/2]` numbered stream. The root's id is unchanged by stacking this
    /// commit, so the single-commit `patch` golden stays byte-identical.
    pub texts_head: ObjectId,
    /// The head commit of the `binmix` branch — the binary `patch` golden's `h`. A
    /// commit modifying one text file and one binary file. `git format-patch` (what
    /// gitweb streams) embeds the binary file as a base85 `GIT binary patch` of
    /// git's own zlib output, which the gix-only, no-unsafe port cannot reproduce;
    /// our port emits git's `--no-binary` form instead (the `Binary files … differ`
    /// notice with an abbreviated `index`). So this commit's golden pins the parts
    /// we DO match (mailbox header, the `Bin <old> -> <new> bytes` diffstat, the
    /// text file's diff, the signature) and documents the binary-body divergence.
    /// Off `HEAD` like the other branches, so every existing golden is untouched.
    pub binmix_head: ObjectId,
    /// The middle commit of the `anchored` branch — the ancestor-named
    /// `commitdiff_plain` golden's `h`. It carries no tag of its own; the
    /// annotated `v2.0` at the branch tip names it `v2.0~1`, the `~N`
    /// ancestor-distance form gitweb stamps on the `X-Git-Tag` line. Off its own
    /// root, so every existing golden is byte-identical.
    pub anchored_ancestor: ObjectId,
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

    /// The `texts` branch's two text files, created by its one commit — the
    /// surface a single-commit `patch` golden frames: the mailbox header, the
    /// multi-file diffstat (name/number column alignment, `create mode` lines),
    /// and the create diff, all byte-for-byte against `git format-patch`.
    pub const TEXT_GREETING: &'static str = "greeting.txt";
    /// The second text file the `texts` root commit creates.
    pub const TEXT_PROGRAM: &'static str = "prog.py";
    /// The text file the second `texts` commit adds — so the range `patches`
    /// golden's `[PATCH 2/2]` carries a create diff alongside the `greeting.txt`
    /// modification, exercising a multi-file mixed diffstat.
    pub const TEXT_NOTES: &'static str = "notes.md";

    /// The `binmix` branch's text file, modified by its head commit — the diff
    /// whose hunk and `name | N +-` diffstat row our binary `patch` golden pins
    /// byte-for-byte against gitweb, proving the non-binary parts still match.
    pub const BINMIX_TEXT: &'static str = "notes.txt";
    /// The `binmix` branch's small binary file, modified by its head commit. Its
    /// pre/post images are too small and dissimilar for a delta to win, so its
    /// `GIT binary patch` body is a `literal` on both the forward and reverse
    /// blocks — the literal-wins arm of the byte-for-byte binary `patch` golden.
    pub const BINMIX_BINARY: &'static str = "zdata.bin";
    /// The `binmix` branch's large, mostly-similar binary file, modified by its
    /// head commit (one interior byte changed). Its pre/post images are big enough
    /// and similar enough that git's binary delta beats the deflated literal, so
    /// its `GIT binary patch` body is a `delta` on both blocks — the delta-wins arm
    /// that actually exercises the diff-delta.c port (gitweb_in_rust-ygu).
    pub const BINMIX_DELTA: &'static str = "bigdata.bin";
    /// The `binmix` branch's binary file *created* by its head commit (absent from
    /// the root). With no pre-image the forward block is a `literal` of the new
    /// blob and the reverse block is a `literal 0` of the empty pre-image — the
    /// binary-create arm of the byte-for-byte binary `patch` golden.
    pub const BINMIX_CREATE: &'static str = "newbin.bin";

    /// The single file the `anchored` branch carries, modified along its chain so
    /// each commit has a real diff for its ancestor-named commitdiff_plain golden.
    pub const ANCHOR_FILE: &'static str = "anchor.txt";

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

/// The text-only `texts` branch the format-patch goldens are captured over: a
/// root commit creating two text files, then a second commit modifying one and
/// adding a third. The root makes `git format-patch -1 --root` a `--root` create
/// diff (`/dev/null` from-side, `create mode` summary lines), with two files of
/// different name lengths exercising the diffstat's column alignment. The range
/// `git format-patch --root <tip>` emits both as a numbered `[PATCH 1/2]` /
/// `[PATCH 2/2]` stream, the second carrying a `greeting.txt` modification hunk
/// next to a `notes.md` create — a multi-file mixed diffstat. No binary file, so
/// the whole stream — headers, diffstats, diffs, signatures — is reproducible
/// byte-for-byte. It hangs off a branch other than `HEAD`, leaving every other
/// golden untouched.
///
/// Returns the root and head commit ids — the single-`patch` golden's `h` and the
/// range `patches` golden's `h`. Stacking the second commit leaves the root's id
/// (and so the single golden) byte-identical.
fn build_texts_branch(builder: &RepoBuilder, who: &Identity) -> (ObjectId, ObjectId) {
    let root_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::TEXT_GREETING.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"hello\nworld\n"),
        },
        TreeEntry {
            name: Corpus::TEXT_PROGRAM.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"print('hi')\n"),
        },
    ]);
    let root: ObjectId = builder.commit(&CommitSpec {
        tree: root_tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "Add greeting and program\n\nThe first text-only commit.\n".to_owned(),
    });

    let head_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::TEXT_GREETING.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"hello\nbright world\n"),
        },
        TreeEntry {
            name: Corpus::TEXT_NOTES.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"# Notes\n"),
        },
        TreeEntry {
            name: Corpus::TEXT_PROGRAM.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"print('hi')\n"),
        },
    ]);
    let head: ObjectId = builder.commit(&CommitSpec {
        tree: head_tree,
        parents: vec![root],
        author: who.clone(),
        committer: who.clone(),
        message: "Revise greeting and add notes\n\nThe second text-only commit.\n".to_owned(),
    });
    builder.branch("texts", head);
    (root, head)
}

/// The `binmix` branch the byte-for-byte binary `patch` golden is captured over: a
/// root commit creating a text file and two binary files, then a head commit that
/// modifies the text file and both binary files and creates a third binary file.
/// The head's `git format-patch` mail mixes a text diff (`notes.txt | 2 +-`) with
/// three `GIT binary patch` bodies covering every arm the port must reproduce:
/// `zdata.bin` (literal wins on both blocks), `bigdata.bin` (delta wins on both —
/// the diff-delta.c arm), and `newbin.bin` (a create: `literal` then `literal 0`).
/// Each binary file carries a full `index`; the text file stays abbreviated. The
/// branch hangs off no other ref, leaving every other golden untouched.
///
/// Returns the head commit id — the binary `patch` golden's `h`.
fn build_binmix_branch(builder: &RepoBuilder, who: &Identity) -> ObjectId {
    let root_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::BINMIX_TEXT.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"first line\nsecond line\n"),
        },
        TreeEntry {
            name: Corpus::BINMIX_BINARY.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&[0x00, 0x01, 0x02, 0x03, 0x04]),
        },
        TreeEntry {
            name: Corpus::BINMIX_DELTA.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&delta_wins_blob()),
        },
    ]);
    let root: ObjectId = builder.commit(&CommitSpec {
        tree: root_tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "binmix base\n".to_owned(),
    });

    let head_tree: ObjectId = builder.tree(&[
        TreeEntry {
            name: Corpus::BINMIX_TEXT.to_owned(),
            mode: Mode::File,
            oid: builder.blob(b"first line\nSECOND line\n"),
        },
        TreeEntry {
            name: Corpus::BINMIX_BINARY.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
        },
        TreeEntry {
            name: Corpus::BINMIX_DELTA.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&delta_wins_blob_modified()),
        },
        TreeEntry {
            name: Corpus::BINMIX_CREATE.to_owned(),
            mode: Mode::File,
            oid: builder.blob(&[0x00, 0x01, 0x02, 0x03]),
        },
    ]);
    let head: ObjectId = builder.commit(&CommitSpec {
        tree: head_tree,
        parents: vec![root],
        author: who.clone(),
        committer: who.clone(),
        message: "binmix head\n".to_owned(),
    });
    builder.branch("binmix", head);
    head
}

/// The `anchored` branch the ancestor-named `commitdiff_plain` golden frames:
/// `ar <- a1 <- a2`, with an annotated tag `v2.0` at the tip `a2`. The middle
/// commit `a1` is named by no tag of its own — only as an ancestor of `v2.0` —
/// so `git name-rev --tags` calls it `v2.0~1` (the annotated `^0` stripped
/// before the generation suffix), which gitweb stamps on its commitdiff_plain
/// `X-Git-Tag` line. This is the parity proof for ancestor-distance naming, the
/// case the old distance-zero subset omitted. Off its own root, so the tag
/// reaches no other commit and every existing golden stays byte-identical.
///
/// Returns the middle commit `a1` — the ancestor-named golden's `h`.
fn build_anchored_branch(builder: &RepoBuilder, who: &Identity) -> ObjectId {
    let commit = |tree: ObjectId, parents: Vec<ObjectId>, message: &str| -> ObjectId {
        builder.commit(&CommitSpec {
            tree,
            parents,
            author: who.clone(),
            committer: who.clone(),
            message: format!("{message}\n"),
        })
    };
    let tree_at = |content: &[u8]| -> ObjectId {
        builder.tree(&[TreeEntry {
            name: Corpus::ANCHOR_FILE.to_owned(),
            mode: Mode::File,
            oid: builder.blob(content),
        }])
    };

    let ar: ObjectId = commit(tree_at(b"base\n"), Vec::new(), "anchored root");
    let a1: ObjectId = commit(tree_at(b"revised\n"), vec![ar], "anchored middle");
    let a2: ObjectId = commit(tree_at(b"final\n"), vec![a1], "anchored tip");
    builder.branch("anchored", a2);
    let _tag: ObjectId = builder.annotated_tag(&TagSpec {
        name: "v2.0".to_owned(),
        target: a2,
        target_kind: TargetKind::Commit,
        tagger: who.clone(),
        message: "release 2.0\n".to_owned(),
    });
    a1
}

/// The `bigdata.bin` pre-image: 600 bytes of a low-period ramp (so it deflates
/// small, making the literal large) with the first byte zeroed (a NUL, so git
/// treats it as binary). Its near-identical [`delta_wins_blob_modified`] post-image
/// makes git's binary delta beat the deflated literal — the delta-wins arm.
fn delta_wins_blob() -> Vec<u8> {
    let mut blob: Vec<u8> = (0..600u32)
        .map(|index: u32| ((index * 7 + 3) & 0xff) as u8)
        .collect();
    blob[0] = 0;
    blob
}

/// The `bigdata.bin` post-image: [`delta_wins_blob`] with one interior byte bumped,
/// so the binary delta is a copy / one-byte insert / copy — far smaller than the
/// deflated literal.
fn delta_wins_blob_modified() -> Vec<u8> {
    let mut blob: Vec<u8> = delta_wins_blob();
    blob[300] = blob[300].wrapping_add(1);
    blob
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

    // The text-only `texts` branch the format-patch (`patch` / `patches`) goldens
    // frame; like the `diffs` branch it hangs off no other ref, so HEAD and every
    // existing golden stay byte-identical.
    let (text_commit, texts_head): (ObjectId, ObjectId) = build_texts_branch(&builder, &who);

    // The `binmix` branch the binary `patch` golden frames — a text+binary commit.
    // Off HEAD like the others, so every existing golden stays byte-identical.
    let binmix_head: ObjectId = build_binmix_branch(&builder, &who);

    // The `anchored` branch the ancestor-named commitdiff_plain golden frames —
    // a commit named only as an ancestor of the `v2.0` tag. Off its own root, so
    // its tag reaches no other commit and every existing golden is untouched.
    let anchored_ancestor: ObjectId = build_anchored_branch(&builder, &who);

    pin_metadata(&repo_path);

    Corpus {
        project_root: project_root.to_path_buf(),
        repo_path,
        blobs,
        diff_parent,
        diff_head,
        text_commit,
        texts_head,
        binmix_head,
        anchored_ancestor,
    }
}
