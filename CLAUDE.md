# Spec & Verification Policy (merge into project CLAUDE.md)

## The check-modification boundary (non-negotiable)

You may make a failing check pass ONLY by changing the implementation.
You may NEVER make a check pass by changing the check.

"Weakening a check" includes ALL of the following:
- Deleting or commenting out a test, scenario, or assertion
- Adding `#[ignore]`, `#[cfg(never)]`, or skipping a test in any way —
  with exactly ONE sanctioned exception: a slice's acceptance test marked
  `#[ignore = "WIP(REQ-<AREA>-<NNN>)"]` per the slice-workflow skill,
  mechanically tracked and failed-if-stale by the audits
- Loosening an assertion (`assert_eq!` → `assert!`, exact value → range, etc.)
- Narrowing a property test's input strategy to dodge a failing case
- Adding a failing input to a "known exceptions" list
- Lowering the mutation-testing threshold or excluding files from `cargo-mutants`
- Raising the CRAP threshold, or padding coverage with weak/assertion-free
  tests to shrink a function's CRAP score
- Editing tests during a refactor step (refactoring changes structure, never
  behavior — if tests must change, the tests are coupled to implementation;
  stop and report)
- Replacing the unit under test with a mock/stub of itself
- Deleting or editing entries in `proptest-regressions/` (these are
  append-only archives of known counterexamples)
- Hand-editing `.fn-hashes.jsonl` (the mutation-scoping snapshot) or adding
  entries to `hex-lint-exceptions.toml` — the snapshot is regenerated only
  by running `fn-hash`; faking a function as "unchanged" skips its
  re-mutation, which is precisely the evasion this list exists to name
- Regenerating or editing golden/conformance files to make a failing parity
  test pass — re-blessing output redefines "correct" and is always a
  check-change (testing-strategy: golden-master rules)
- Deleting and regenerating a module, test file, or function to escape a
  failing check instead of fixing forward — rewriting is not fixing, and it
  silently discards behavior other tests don't pin
- Relaxing a type (newtype → primitive, `NonZeroU32` → `u32`, removing a validation)

If you believe a check is genuinely wrong: STOP. Do not touch it. Report to the
human: which check, why you believe it is wrong, what you propose instead.
A check change is always a separate, human-approved commit — never bundled
into a feature or fix commit.

## Traceability (every test has a reason to exist)

- Every requirement lives in `specs/` and has an ID: `REQ-<AREA>-<NNN>`.
- Every test function references exactly the requirement IDs it verifies,
  in a comment on the line above the test attribute:
  `// REQ-SALE-003`
- Do not write tests with no requirement ID. If behavior needs a test but no
  requirement exists, write the requirement first (see skill: spec-authoring).
- Do not write requirements that nothing verifies. A requirement merged
  without at least one referencing test is incomplete work.
- Before changing any requirement: `rg 'REQ-<ID>'` and update every hit,
  spec and tests together, in the same commit.

## Order of work for any new behavior

1. Requirement (EARS) + feature name + anchor scenario → `specs/`
   (skill: spec-authoring)
2. Types first: make illegal states unrepresentable before writing logic.
3. Implement via red/green/refactor (skill: tdd-cycle), under the Three Laws:
   - No production code except to pass a failing test; at most ONE failing
     test in existence at a time; no more code than the test forces.
   - RED: see the new test fail, for the expected reason, before any
     production code.
   - GREEN: minimum code to pass; all tests green.
   - REFACTOR: mandatory every cycle, on green only, tests untouched.
     Commit on green.
4. `cargo mutants` scoped via `fn-hash --changed-only` (function-granular,
   fmt-immune). Triage every surviving mutant (skill: verification-ratchet).
   Surviving mutants are reported, never hidden. Hash-gated scoping is an
   approximation — full runs in CI remain the proof. ALWAYS run cargo-mutants
   with `TMPDIR=$HOME/.cache/cargo-mutants` (the default `/tmp` is a 16 GB
   tmpfs; cargo-mutants copies the whole `target/` per mutant and overflows it,
   reporting every mutant "unviable" — a false green). See verification-ratchet.
5. Quality gate: hex-lint clean (role matrix), no changed function over
   CRAP 30. Over-threshold functions are refactored or reported — never
   accepted silently and never gamed via coverage padding
   (skill: verification-ratchet, Layer 4).
6. ADR if a structural decision was made (skill: adr-discipline).

## Vertical slices (the double loop)

When one requirement spans multiple zones (domain, port, adapter, use case,
web/render), follow the slice-workflow skill:
- ONE behavioral REQ owns the slice; layers are not requirements.
- ONE acceptance test per in-flight slice, committed `#[ignore = "WIP(REQ-…)"]`
  on day one, un-ignored as the final wire-up step. Removing the marker and
  seeing it green IS the slice's definition of done.
- The one-failing-test rule and "never end mid-red" apply to the INNER
  (unit) loop; the WIP-ignored acceptance test is the outer loop and is
  allowed to stay red — visibly, trackably — for the slice's lifetime.
- Layer order default: spec → WIP acceptance → domain → port + contract
  suite + fake → use case (against fake) → adapters (passing the same
  contract suite) → web/render → wire-up → gates.
- First slice in a new capability is a walking skeleton: thinnest behavior
  through all layers first.

## Getting stuck

If the next test cannot pass without a large rewrite, do not brute-force it
and do not weaken the test. Revert to the last green commit, report the
blockage and a proposed re-sequencing or design change, and wait
(skill: tdd-cycle, backtracking protocol).

## Baseline rule

The suite must be green before you write anything. If orientation finds a
red baseline, that is inherited breakage: stop, report it, and wait — never
build on red, or your failures and the previous session's become
indistinguishable. Never run interactive/TUI commands or watch modes; use
non-interactive flags (`--json`, `--no-pager`) or report the tool as
unusable.

## Evidence discipline (claims require receipts)

- Never state that tests pass, a mutant is killed, a gate is green, or a
  build succeeds unless you ran the command IN THIS SESSION and saw the
  output. "Should pass" is not "passes".
- When declaring a step done, show the evidence: the command and the
  relevant tail of its output (test count, mutant summary, gate result).
- A RED step requires showing the failure output and confirming it failed
  for the expected reason — a test you never saw fail is not evidence.
- If a command was not run (timeout, environment problem), say exactly that.
  An honest "could not verify" is acceptable; a fabricated "verified" is the
  single worst thing you can do in this codebase.

## Scope discipline

- Touch only files the current requirement forces you to touch. No drive-by
  refactors, renames, dependency bumps, or "while I'm here" cleanups outside
  the REFACTOR step of the code you are working on — file them instead
  (see: Structural improvements, below). Discovery is encouraged; silent
  scope creep is not.
- If implementing REQ-X reveals a bug or gap elsewhere: report it, propose a
  requirement, do not silently fix it in the same change.
- Diff size is a review burden you impose on the human. Prefer several small
  green commits over one large one; the commit history should replay the
  red/green/refactor sequence.

## Session lifecycle (first and last steps are mandatory)

- START of session and after any compaction: run the orientation protocol
  (skill: session-lifecycle). Read this file, check git state, read
  HANDOFF.md and the assigned bead, run trace-audit, state which REQ you're
  working on. Do not rely on a summary's paraphrase of these rules.
- END of session: handoff protocol (skill: session-lifecycle). Never end
  mid-red; gates green; bead updated; HANDOFF.md rewritten with the exact
  next test. Anything not committed to the repo does not exist tomorrow.
- Beads specify WHAT, never HOW. A bead references REQ IDs and gives
  context; implementation decisions belong to the session doing the work,
  inside the constraints of specs and ADRs.

## Project conventions (mechanically enforced where possible)

- Hexagonal architecture, ECB flavor: Entities hold business logic,
  Controls implement use cases via ports, Boundaries are adapters
  (API, CLI, DB). All dependencies point inward. No framework types in
  the domain. This is MECHANICALLY enforced by hex-lint: every workspace
  member carries `[package.metadata.hex-arch] role = "..."` (domain,
  usecase, port-and-adapter, driven-adapter, driving-adapter, infra,
  composition-root) and the gate fails on any forbidden cross-role edge.
  A missing role tag is a hard error. Adding an entry to
  `hex-lint-exceptions.toml` is a check-change: standalone commit, human
  approval — and stale exceptions fail the lint on their own, so debt
  cannot be papered over and forgotten.
- Screaming architecture: organize modules by business capability, not by
  technical layer. The REQ `<AREA>` prefix, the bounded context, and the
  module directory should share a name — `specs/REQ-PRICING-*` lives in
  `src/pricing/`. If you can't tell what the system does from `ls src/`,
  the structure is wrong.
- `#![forbid(unsafe_code)]` in every domain and application crate. If a
  boundary crate genuinely needs `unsafe` (embedded HAL, FFI), that is an
  ADR with human approval — never a quiet `#[allow]`.
- Tests have cyclomatic complexity 1: straight-line bodies, no loops, no
  branches. Case coverage follows zero/one/many — and two counts as many.
  A loop over a table in a test is either separate test functions, rstest
  cases, or a property. A branch in a test means you don't know what the
  test asserts.
- Explicit type annotations on variables and closure parameters. The type
  system is a spec; make it visible at the point of use.
- Commits: scoped style, `<scope>: <description>` — scope is the
  capability/module touched, not a generic type. REQ IDs in the body; when
  a tracker is in use, the message ends with the bead/work-item id —
  commit ↔ bead ↔ REQ is the traceability triangle. `check-change:` is the
  reserved scope for human-approved check modifications (check-guard
  enforces it).

## Structural improvements (the senior-dev override, bounded)

When you find flawed architecture, duplicated state, or inconsistent
patterns, you do not ignore it and you do not silently fix it. The rule:

- Inside the code you are already changing for the current REQ: fix it in
  the REFACTOR step, as its own scoped commit. That's what the step is for.
- Outside the current REQ's blast radius: file it — a bead (and a spec
  revision or ADR proposal if it's structural). Filing aggressively is
  mandatory; fixing opportunistically is forbidden. A "senior dev" who
  bundles an unrelated restructuring into a feature diff gets the whole
  diff rejected, and so do you.
- Step 0 rule: before any structural refactor of a file >300 LOC, first
  strip dead code — unused imports/exports, dead props, debug logs — as a
  separate commit. Dead code poisons context compaction and inflates every
  diff; clean the lens before grinding it.

## Mutation testing outcomes (the only allowed responses)

For each surviving mutant, exactly one of:
- **Strengthen**: add/extend a test (with a REQ ID) that kills it.
- **Report**: tell the human "this mutant survives because the spec does not
  constrain this behavior — should it?" and wait.
Never: exclude the file, lower the threshold, or declare it unimportant
on your own authority.

## ADRs are settled law

Before proposing any architectural or modelling change, search `docs/adr/`.
If an accepted ADR already covers it, do not relitigate — surface the ADR to
the human and proceed within it unless the human supersedes it.
