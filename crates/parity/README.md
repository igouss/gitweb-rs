# gitweb-parity — golden differential conformance

gitweb's HTML pages are modernized in gitweb-rs, so they are verified
behaviourally. Its **format-stable** endpoints are not: other tools parse them,
so they must match the original `gitweb.perl` **byte for byte**. This crate pins
that parity with golden differential conformance.

- `blob_plain`, `commitdiff_plain`, `patch`, `patches`
- `project_index`, `opml`
- `rss`, `atom`
- `blame_data`

## How it works

1. **One deterministic corpus.** [`corpus::build`](src/corpus.rs) writes a bare
   `repo.git` with gix. The SAME builder runs at capture time (via the
   `build-corpus` binary) and at test time, so every object id matches and the
   goldens never flicker.
2. **Capture once.** [`scripts/capture-goldens.sh`](scripts/capture-goldens.sh)
   assembles a real `gitweb.cgi` from the pinned upstream `gitweb.perl`, drives
   it over the corpus, and freezes each raw CGI response under
   [`goldens/`](goldens). These files are committed; **neither perl nor git runs
   at test time.**
3. **Diff our output.** The `golden` cucumber target rebuilds the corpus, reads
   our format-stable output through the real gix adapter, and asserts it is
   byte-identical to the captured reference. [`Golden`](src/golden.rs) splits the
   CGI header block from the body, binary-safe.

Run the conformance test:

```sh
cargo test -p gitweb-parity --test golden
```

## Refreshing the goldens

Only needed when the corpus changes or a new endpoint is wired. Requires a perl
with the CGI module and the git source tree (the `gitweb.perl` source of truth):

```sh
# one-time: a perl that has CGI (system perl 5.22+ dropped it from core)
brew install perl cpanminus && cpanm CGI

# capture (override GIT_SRC / PERL / GIT if they are not where it expects)
sh crates/parity/scripts/capture-goldens.sh
```

The script is reproducible: re-running it produces byte-identical goldens.
`blob_plain` is captured by tree path (`f=…;hb=HEAD`), not by raw hash, because
the by-hash path makes gitweb stamp volatile `Expires`/`Date` cache headers that
would make the golden non-reproducible.

## Wiring a new endpoint

1. Extend `corpus::build` if the endpoint needs new fixture content.
2. Add a `capture …` line to `capture-goldens.sh` and re-run it.
3. Add a feature under `features/golden/` and steps in `tests/golden.rs` that
   produce our output and assert it against the captured body (and, where the
   endpoint's headers are format-stable, its headers via `Golden::header`).
