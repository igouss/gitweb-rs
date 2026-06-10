---
name: verification-ratchet
description: Use when implementing a requirement, writing tests, deriving property tests from a spec's candidate laws, running and triaging cargo-mutants results, or checking the CRAP complexity gate. Defines what counts as real verification and the only legal responses to failing checks, surviving mutants, and CRAPpy functions.
---

# Verification Ratchet

The ratchet only moves one direction: checks get stronger, never weaker.
The check-modification boundary in CLAUDE.md overrides everything here.
This skill defines WHAT the checks are; the tdd-cycle skill defines the
red/green/refactor loop you walk while building them.

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
# function-granular incremental scoping via fn-hash (preferred):
fn-hash --changed-only | sed 's/ :: .*//' | sort -u \
  | xargs -I{} cargo mutants --file {}
cargo mutants            # full run: CI on the dev branch / before merge
```

fn-hash hashes every function over its normalized token stream (so `cargo
fmt` is not a "change") against the committed `.fn-hashes.jsonl` snapshot,
and prints only the units that moved. The snapshot advances via the
pre-commit hook running `fn-hash` — it is NEVER hand-edited (protected
check infrastructure; faking "unchanged" skips re-mutation of the code you
just wrote, the canonical evasion).

Two blind spots you must respect, neither excusable as evidence:
- Hash-gated scoping is an approximation: a change in function A can alter
  which mutants in UNCHANGED function B are caught (callers shift which
  paths tests exercise; generics/inlining couple units). Full runs in CI
  remain the proof; scoped runs are fast feedback only.
- Scoped/`--in-diff` runs match changes against PRODUCTION code only — a
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

Run it after the mutation pass (see `scripts/quality-gate.bb`):

```bash
cargo llvm-cov --lcov --output-path target/lcov.info   # coverage first
cargo crap          # check `cargo crap --help` for flags/threshold config
```

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
