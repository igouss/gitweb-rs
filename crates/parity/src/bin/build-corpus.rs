//! Lays the parity corpus on disk and prints its manifest.
//!
//! Run by `scripts/capture-goldens.sh` before it drives `gitweb.perl`: it writes
//! `repo.git` under the given project root and prints one `name <oid> <file>`
//! line per blob to stdout, so the capture script knows the tree path each golden
//! is addressed by. It shares [`gitweb_parity::corpus::build`] with the golden
//! test, so the object ids printed here are the ids the test reads back.
//!
//! Usage: `build-corpus <project-root-dir>`

use std::path::PathBuf;
use std::process::ExitCode;

use gitweb_parity::corpus::{self, BlobFixture, Corpus};

fn main() -> ExitCode {
    let Some(root): Option<String> = std::env::args().nth(1) else {
        eprintln!("usage: build-corpus <project-root-dir>");
        return ExitCode::FAILURE;
    };

    let corpus: Corpus = corpus::build(&PathBuf::from(root));
    for fixture in &corpus.blobs {
        let BlobFixture {
            name,
            oid,
            file_name,
        } = fixture;
        println!("{name} {oid} {file_name}");
    }
    ExitCode::SUCCESS
}
