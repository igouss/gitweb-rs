//! The deterministic fixture repository the goldens are captured over.
//!
//! [`build`] writes one bare repository, `repo.git`, under a caller-owned
//! project root. It is the single source of truth for the corpus: the
//! `build-corpus` binary runs it to lay the repository down for `gitweb.perl` to
//! read at capture time, and the golden test runs it to rebuild the very same
//! repository for the gix adapter to read at test time. Because the builder is
//! deterministic, both constructions produce identical object ids, so a golden
//! captured for a blob corresponds exactly to the blob the test reads back.

use std::path::{Path, PathBuf};

use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId, RepoBuilder, TreeEntry};

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
}

impl Corpus {
    /// The git project name `gitweb.perl` addresses this repository by (`p=`),
    /// relative to the project root.
    pub const PROJECT: &'static str = "repo.git";

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
        committer: who,
        message: "corpus root\n".to_owned(),
    });
    builder.branch("main", root);
    builder.set_head("main");

    Corpus {
        project_root: project_root.to_path_buf(),
        repo_path,
        blobs,
    }
}
