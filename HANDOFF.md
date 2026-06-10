# Handoff

## Last session: gitweb_in_rust-23u DONE (object/dispatch gitlink base+path)

**What shipped** — a mode-160000 gitlink/submodule named via base+path now
resolves correctly through both consumers, FAITHFUL to gitweb (verified by
reading the subs + empirical git):

- **object action** redirects to `a=commit` (gitweb takes `$type` from
  `ls-tree`'s mode-derived type column, which prints "commit" for a gitlink,
  even though the submodule commit is absent here). FIXED.
- **no-action dispatch** 404s "File or directory does not exist" (gitweb's
  `git_get_type` = `cat-file -t hb:f` reads the absent object and fails). Was
  ALREADY faithful — no code change, only a characterization scenario.

WARNING: Bead 23u's premise that dispatch reports "commit" for a gitlink was
WRONG. See memory `gitlink-object-vs-dispatch-asymmetry`. No follow-up needed.

**New seam**: `Repository::path_entry(at, path) -> Option<TreeEntry>` — the
ls-tree ROW (mode + name + oid); `path_id` now delegates to it (gix + fake).
Classify a path by `entry.mode().object_kind()`, not by cat-file'ing a maybe
-absent id.

**Commits**: a5ef39f (port+gix conformance) - 4a1a5d1 (object usecase) -
793c990 (dispatch characterization) - 463ad8d (web acceptance).

**Gates**: fmt OK - clippy --workspace --all-targets OK - full workspace tests
green - cargo-mutants scoped to the 3 changed prod fns = 0 survivors
(redirect_by_base_path + path_id/path_entry: caught or unviable). CRAP not
mechanically run (whole-workspace llvm-cov); the 3 fns are CC<=3 and fully
covered, so far under 30.

**Pre-existing, NOT mine**: `hex-lint` reports every workspace package is
`missing package.metadata.hex-arch.role` (the role matrix was never set up).
hex-lint exits 0 so it doesn't fail the gate, but the architecture gate is
effectively unenforced. Worth a bead (likely under 9uc / CI). `fn-hash` has no
`.fn-hashes.jsonl` baseline, so `--changed-only` reports the whole repo —
mutation scoping was done by hand this session.

## Next session

Baseline is green. Suggested ready beads (P3 first):
- **d7s** (P3): thread `get_branch_refs` into the heads listing (' (dir)'
  suffix) + last-activity. Needs `references` to take multiple prefixes — the
  meatiest. See memory `branch-refs-seam`.
- **12k** (P4, small): forks dir-exists-but-zero-forks unlinked '+' span;
  needs a ProjectStore container-exists port method. See `forks-view-seams`.
- **e5l** / **him**: golden-parity beads (need brew perl+CGI to regenerate
  gitweb refs).

The graph top-picks (h2t client-JS, 2os.13 combined --cc) are large
multi-session capabilities.

### Exact next test if picking 12k
Add to `crates/domain/features/usecase/project_list.feature` a scenario: a
fork-capable project whose sibling container dir exists but folds zero forks
renders the unlinked `+` span (state 2). Watch it red against the current
`partition_forks` (which collapses state-2 into state-1, no '+'), then thread
a `ProjectStore::container_exists`-style port read behind the Forks gate.
