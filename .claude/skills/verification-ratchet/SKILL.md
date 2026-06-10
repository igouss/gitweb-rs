---
name: verification-ratchet
description: Use when implementing a requirement, writing tests, deriving property tests from a spec's candidate laws, running and triaging cargo-mutants results, or checking the CRAP complexity gate. Defines what counts as real verification and the only legal responses to failing checks, surviving mutants, and CRAPpy functions.
---

# Verification Ratchet

The ratchet only moves one direction: checks get stronger, never weaker.
The check-modification boundary in CLAUDE.md overrides everything here.
This skill defines WHAT the checks are; the tdd-cycle skill defines the
red/green/refactor loop you walk while building them; the quality-gates
skill defines the machinery that runs them — including the failure mode
this skill's layers cannot see: a gate that silently stopped firing.
A ratchet is only monotone if its *firing* is observable.

## Layer 0 — Types (cheapest, do first)

Before any test: can the illegal state be made unrepresentable?

- Domain quantities are newtypes, not primitives: `Money`, `Percent`, `Quantity`.
- Constructors validate; the inner value is private. `Percent::new(150)` fails
  at the boundary; everything past the boundary trusts the type.
- Prefer `NonZero*`, enums over booleans-with-meaning, and typestate for
  must-happen-in-order protocols at the hexagonal ports.
- `#![forbid(unsafe_code)]` at the crate root of every domain/application
  crate — the cheapest check in the entire kit: one line, compiler-enforced,
  and `forbid` (unlike `deny`) cannot be overridden by an inner `#[allow]`.
  Boundary crates that truly need unsafe (FFI, embedded HAL) get it via an
  ADR, scoped to that crate, never the domain.

Every state the compiler forbids is a scenario you do not write and a mutant
that cannot survive. If a candidate Gherkin scenario describes constructing an
invalid value, the correct implementation is a type, and the scenario becomes
a one-line constructor test.

The same principle one level up: **prefer mechanisms where the wrong
state is uncompilable over mechanisms where it is merely labeled.** A
capability only some implementations support is a separate supertrait
with NO default methods — a default impl lets every backend type-check
into the path and degrade silently at runtime; the no-default split
makes the wrong wiring a compile error. A metadata label ("this crate is
domain") checked by a lint verifies only the structural consequence, not
the premise — the type system is the one checker that verifies the
premise (quality-gates skill).

## Layer 1 — Example tests (from anchor scenarios)

- One test per anchor scenario, asserting the exact values in the scenario.
- Tests have cyclomatic complexity 1: straight-line bodies — no `if`, no
  `match`, no loops. A branch in a test means the test doesn't know what it
  asserts; a loop over cases hides which case failed. Tables become rstest
  cases or separate functions; "for all" becomes a property (Layer 2).
  Mechanical option: `#![deny(clippy::cognitive_complexity)]` on test
  modules with `cognitive-complexity-threshold = 1` in clippy.toml.
- Case selection follows zero/one/many: empty input, single element, several
  — and two counts as many. If the spec's scenarios don't cover all three
  where applicable, that's a spec gap (spec-authoring), not extra tests to
  invent silently.
- Name encodes the scenario: `fn sale_with_percentage_discount()`.
- REQ ID comment on the line above the attribute:
  ```rust
  // REQ-SALE-003
  #[test]
  fn sale_with_percentage_discount() { ... }
  ```
- Test the port, not the adapter. No mocking the unit under test, ever.
  Adapters get their own thin integration tests.

## Layer 2 — Property tests (the laws)

Take the spec's `## Candidate laws` and implement with `proptest`. Derivation
patterns, in order of usefulness:

1. **Invariant**: an inequality or relation that always holds.
   `total <= subtotal` for any discount.
2. **Identity / neutral element**: `discount(0%)` changes nothing;
   `serialize ∘ deserialize == id` (round-trip).
3. **Metamorphic relation**: relate two calls without knowing either answer.
   `discount(a+b items) == discount(a) + discount(b)` if linear.
4. **Oracle**: compare against a slow-but-obviously-correct implementation
   for small inputs.

Rules:
- Strategy ranges come from the spec ("0–100 inclusive" → `0u8..=100`),
  never from what makes the test pass.
- `proptest-regressions/` files are COMMITTED to git, always. proptest writes
  failing seeds there and replays them before generating new cases — they are
  the project's archive of known counterexamples. They are append-only:
  deleting or editing an entry is check-weakening (CLAUDE.md).
- Keep properties deterministic-enough for CI: the default case count is
  fine; if a property is slow, reduce work per case (smaller collections via
  the strategy), don't reduce assurance by gutting the strategy's range.
- On a failing property: minimize the counterexample, then decide —
  implementation bug (fix code) or spec gap (STOP, report to human with the
  counterexample; do not adjust the strategy).
- Property tests carry REQ IDs like example tests.
- See the testing-strategy skill for WHERE properties belong (domain core
  and port round-trips — not adapters).

## Layer 3 — Mutation testing (verifies the verification)

After implementation is green:

```bash
# ALWAYS run cargo-mutants with TMPDIR on a real disk, NEVER the default /tmp.
# cargo-mutants copies the whole workspace + target/ into $TMPDIR for every
# mutant build; on this host /tmp is a 16 GB tmpfs, so the copy overflows it
# ("Disk quota exceeded (os error 122)") and EVERY mutant is reported
# "unviable" — a false all-green that proves nothing. /home has the space.
export TMPDIR="$HOME/.cache/cargo-mutants"; mkdir -p "$TMPDIR"

# incremental scoping — only mutants in code your diff touches (fast feedback):
cargo mutants --in-diff <(git diff)            # uncommitted work
cargo mutants --in-diff <(git diff main...)    # whole branch vs its merge-base
cargo mutants            # full run: CI on the dev branch / before merge
```

If a mutants run ends with "No mutants were viable" or every mutant is
`unviable`, do NOT read that as success — it is almost always the /tmp-tmpfs
overflow above (check `mutants.out/log/*` for "Disk quota exceeded"). Re-run
with `TMPDIR` on `/home` before trusting any result.

`--in-diff` reads a unified diff and restricts mutation to the lines it
touches — finer than per-file and nothing to maintain. One caveat: it sees a
raw diff, so a pure `cargo fmt` reflow reads as "changed" and re-mutates
untouched logic; format, commit, then diff to avoid it.

Two blind spots you must respect, neither excusable as evidence:
- Diff scoping is an approximation: a change in function A can alter
  which mutants in UNCHANGED function B are caught (callers shift which
  paths tests exercise; generics/inlining couple units). Full runs in CI
  remain the proof; scoped runs are fast feedback only.
- `--in-diff` runs match changes against PRODUCTION code only — a
  test-only change triggers zero mutants, so scoped mutation testing cannot
  detect test-weakening. That detection is check-guard.bb's job. Never cite
  a scoped mutants pass as evidence about a test-only change.

Triage EVERY surviving mutant. Exactly two legal outcomes per mutant:

1. **Strengthen** — write or extend a test (with a REQ ID) that kills it.
   If no existing requirement constrains the mutated behavior, that is a spec
   gap: invoke spec-authoring, write the requirement, then the test.
2. **Report** — tell the human: mutant location, the mutation, why no current
   requirement constrains it, and your recommendation. Then wait.

Illegal outcomes: excluding files, lowering thresholds, `#[mutants::skip]`
without explicit human approval recorded in the commit message, or declaring
a mutant "equivalent" on your own authority (claimed-equivalent mutants are
reported, with reasoning, and the human decides).

## Layer 4 — CRAP gate (catches what mutants don't: complexity)

CRAP — Change Risk Anti-Patterns (Savoia & Evans, 2007; championed by Robert
Martin, who uses it as a gate for AI-assisted development). Per function:

```
CRAP(m) = CC(m)² × (1 − cov(m))³ + CC(m)
```

CC = cyclomatic complexity, cov = test coverage fraction. Floor is 1
(CC=1, fully covered). At full coverage the score collapses to bare CC;
at zero coverage it explodes to CC² + CC. Threshold: **30**. Note the
consequence: a function with CC ≥ 31 is over threshold even at 100%
coverage — past that point the only fix is refactoring, by design.

Run it after the mutation pass (see `scripts/quality-gate.bb`).

**Implementation — the kit ships the join** (`templates/crap-report.bb`,
field-proven; no dependency on a young third-party scorer). CRAP needs
per-function coverage × per-function complexity, and no single Rust tool
emits both:

```bash
cargo llvm-cov nextest --workspace --json --output-path target/metrics/coverage.json
./scripts/crap-report.bb --coverage target/metrics/coverage.json [--gate 30]
# the fast pre-commit half (complexity only, staged files):
./scripts/complexity-gate.bb
```

The non-obvious rules, each paid for in the field:

- **Join by file path + line-range overlap, never by function name.**
  Rust name mangling means names do not line up between tools; a
  coverage region counts for a function if its line range overlaps the
  function's. Crude but honest — and it works.
- **Exclude tests/benches/examples from the function set** — tests are
  always "covered" and skew the distribution.
- **Uninstrumented functions score as 0% coverage** (pessimistic), but
  are counted and surfaced separately — they're usually cfg-gated code
  the test target never compiled, which is its own finding.
- **Two speeds, never one**: a fast pre-commit gate checks ONLY
  complexity on staged files (deliberately loose thresholds — it exists
  to catch egregious cases, not to nag every refactor; documented
  override env-var, not `--no-verify`). The full coverage×complexity
  CRAP report is slow and lives behind an explicit recipe
  (`just metrics`), run before merge/release — never in a hook.
- **Trend, not just threshold**: each full run appends one summary row
  (date, commit, total functions, mean/median/max CRAP, mean coverage,
  worst function) to a **committed** `metrics/history.csv`. The ratchet
  direction becomes reviewable: max-CRAP and mean must not drift up
  across the history. Make the append idempotent per (date, commit) —
  the field version wasn't, and duplicate rows got committed.
- **The report teaches**: embed the triage playbook in the generated
  report itself ("1. split or simplify — reduce CC; 2. cover the
  branches; 3. if neither is feasible, note it and move on — don't
  chase the tail"), so the agent reading the report doesn't need this
  skill open.

For every function over threshold, exactly two legal outcomes (mirror of
mutant triage):

1. **Refactor** — reduce CC, almost always Extract Function on the branchy
   parts (tdd-cycle skill, REFACTOR rules apply: on green, tests untouched).
   This is the usual outcome; prefer it.
2. **Report** — if the complexity is essential (parser, state machine,
   protocol handler) and genuinely covered, say so to the human with the
   score and reasoning. The human decides whether it's accepted; you do not.

Illegal outcomes: raising the threshold; padding coverage with weak or
assertion-free tests to shrink the (1 − cov)³ term — that is check-gaming,
it will not survive the mutation pass, and a test added solely to move a
metric has no REQ to trace to.

Division of labor — why this layer is not redundant with Layer 3:
- **cargo-mutants** proves your tests are STRONG (would notice wrong code).
  It says nothing about whether the code is comprehensible.
- **CRAP** flags code that is COMPLEX and under-tested — the refactor signal.
  Its coverage term is line coverage, which is gameable; mutation score stays
  the truth metric for test strength. CRAP's unique contribution is the
  complexity term: it is the only mechanical check in this kit that forces
  the REFACTOR step to actually happen instead of being skipped under
  pressure.

## Smells that mean you are doing verification theater

- A test that restates the implementation (computing the expected value with
  the same algorithm under test).
- Asserting only "does not panic" when the spec states an exact outcome.
- Tests that pass when the function body is replaced by `Default::default()`
  — that is precisely what cargo-mutants will tell you; listen to it.
- Coverage going up while mutation score goes down.
