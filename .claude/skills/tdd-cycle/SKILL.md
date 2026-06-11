---
name: tdd-cycle
description: Use during the implementation step of any requirement — after the spec exists and before declaring done. Defines the red/green/refactor loop, the Three Laws adapted for agents, the mandatory refactor step with its quality gates (Four Rules of Simple Design, CRAP threshold), and the backtracking protocol for when you get stuck.
---

# TDD Cycle (Red / Green / Refactor)

Source discipline: Robert C. Martin's Three Laws of TDD and his Cycles of TDD.
Rationale: a mind (or model) cannot pursue correct behavior and correct
structure simultaneously. So the loop separates them: make it work (red→green),
then make it right (refactor), never both in one move.

## The Three Laws, agent form

1. Write no production code except to make a currently-failing test pass.
2. Write no more of a test than is sufficient to fail
   (compilation failure counts as failing).
3. Write no more production code than is sufficient to pass the one
   failing test.

For an agent, Law 3 is the anti-speculation rule and it is the one you will
be most tempted to break: no "while I'm here" helpers, no handling of inputs
no test demands, no config options nobody asked for, no `pub` that nothing
external calls. Speculative generality is unverified code by construction —
nothing forces it, so cargo-mutants will flag it, and the traceability audit
has no REQ to hang it on. If you think the code will be needed: that is a
requirement; invoke spec-authoring first.

Step size: you may take larger green steps than a human keystroke-level
cycle — one anchor scenario or one property at a time is the right grain —
but never more than ONE failing test in existence at once, and never
production code that no failing test forced.

## The loop, per requirement

Work from the spec's anchor scenarios and candidate laws as your test list,
simplest first.

1. **RED** — write the next test (with its REQ ID). Run it. Confirm it fails,
   and fails for the expected reason. A test you never saw fail proves
   nothing; if it passes immediately, either the behavior already exists
   (check traceability — maybe this scenario is already covered) or the test
   is vacuous (fix it).
2. **GREEN** — minimum production code to pass. All tests green, not just
   the new one. It is fine for this code to be crude; do not polish here.
3. **REFACTOR** — mandatory, on green only, every cycle (it may legitimately
   conclude "nothing to do", but you must look). See below.
4. Next test. When the scenario list and candidate laws are exhausted,
   proceed to mutation testing (verification-ratchet, Layer 3) — **SUSPENDED
   as of 2026-06-10 per CLAUDE.md "Order of work" step 4; skip it for now and
   go straight to the quality gate (Layer 4).**

## RED in a compiled language — stub wrong-but-valid, never todo!()

When the new test references production code that doesn't exist yet, the
honest red is blocked by a compile error — which is not a red, it's a
build failure. The discipline: write the production signature as a
*compiling stub that returns a wrong-but-valid value* (`is_merge -> false`
always, `title -> String::new()`, `parse -> None`). Run the suite → clean
assertion failures you can count and read. Then fill in the real body →
green.

Do NOT stub with `unimplemented!()`/`todo!()`: panicking stubs abort the
run at the first failure instead of giving you the full red picture. The
red count is itself a check on your spec partitioning — N scenarios should
produce N comprehensible failures, and zero failures against a stubbed
region tells you that region carries no behavior worth a spec (it's a data
carrier, not a rule — see testing-strategy on what doesn't get tests).

## REFACTOR — the rules

- Refactor ONLY on green. Tests are your safety net; you may not refactor
  through a red bar.
- Refactoring changes structure, never behavior, and therefore NEVER touches
  tests. If improving the structure seems to require editing tests, the tests
  are coupled to implementation details (testing through the adapter, mocking
  internals, asserting on structure). Stop and report — that coupling is the
  bug, and fixing tests is a check-modification (see CLAUDE.md boundary).
- Criteria, in order (Beck's Four Rules of Simple Design):
  1. Passes all tests (non-negotiable).
  2. Reveals intent — names say what, structure says how the domain thinks.
  3. No duplication of knowledge (not just of text).
  4. Fewest elements — remove anything the first three don't require.
- The three refactorings that do most of the work: Rename, Extract Function,
  Compose Method. Reach for these before anything clever.
- Step 0 for structural refactors of files >300 LOC: first strip dead code —
  unused imports/exports, dead fields, debug logging — as its OWN commit,
  before the real restructuring. Dead code inflates every subsequent diff
  and poisons context compaction; clean the lens first.
- Each refactor commits separately from the green step, scoped-commit style
  (`pricing: extract bracket lookup from compute_tax`), so the history
  replays as red→green→refactor and a reviewer can reject the refactor
  without rejecting the behavior.
- Hard trigger: any function whose CRAP score exceeds the threshold
  (see verification-ratchet, Layer 4) must be refactored or reported in this
  step — usually Extract Function on the branchy parts. Do not carry a
  CRAPpy function forward to the next red.

## Specific tests, generic code

As the tests get more specific, the production code should get more generic.
If you find yourself adding an `if`/`match` arm per test case, you are
hard-coding answers, not generalizing — the discount table grows a branch per
scenario instead of becoming arithmetic. Prefer the simplest transformation
that generalizes (constant → variable → conditional → iteration/recursion —
the Transformation Priority Premise ordering). A pile of special cases is
exactly the shape cargo-mutants shreds.

## Getting stuck (the backtracking protocol)

Symptom: the next test cannot be made green without a large rewrite — you
need to change a lot of production code at once, outside the loop. This means
an earlier green step generalized in the wrong direction.

Per Martin: the only honest fix is to backtrack — revert to an earlier green
state, reorder the remaining tests (usually: you took a too-specific test too
early), and take a different fork. For an agent the protocol is:

1. Do NOT brute-force the rewrite inside one "green" step, and do NOT weaken
   the blocking test (boundary violation).
2. `git stash` / revert to the last green commit.
3. Report to the human: which test blocked you, why the current structure
   can't absorb it, and your proposed re-ordering or design change (this may
   warrant an ADR).
4. Proceed only after the human acks, then re-derive the dropped tests —
   they go back in; backtracking re-sequences work, it never deletes
   requirements coverage.

Commit on green, every cycle, so this protocol is always cheap.
