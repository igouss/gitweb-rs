---
name: quality-gates
description: Use when designing, wiring, debugging, or being blocked by quality-gate infrastructure — git hooks, lint matrices, exception files, complexity gates, Stop hooks. Defines the three-tier latency topology, the vacuous-green problem (gates must prove they FIRED), exception-file hygiene, and gate failure UX. verification-ratchet defines WHAT the checks are; this skill defines how the machinery that runs them stays honest.
---

# Quality Gates — the machinery, and how it lies

Field origin: a 389-commit Rust workspace where every architectural rule
acquired a mechanical gate — and where the three worst incidents were all
the same incident: a green signal that proved nothing.

## Vacuous green — the central failure mode

Green has a failure mode: the check that didn't run. Three independent
field incidents, one rule:

1. **Smoke tests silently self-skipped for weeks** — the skip-guard probed
   for the dependency at a filesystem path; the dependency lived elsewhere;
   all 11 tests skipped and reported green while the adapter was broken in
   two ways against the real binary.
2. **Clippy debt accumulated invisibly** — somewhere in a bead chain
   `--no-verify` or a broken hook slipped through; check+tests stayed green
   (they don't cover lints) while 20+ deny-level errors piled up across 9
   crates.
3. **Spec steps drifted from step definitions** — only caught because the
   BDD runner was configured `fail_on_skipped`: an undefined step is a red
   build, not a silent skip.

The rule: **every gate needs proof that it fired, not just absence of
red.** Consequences:

- A skip-guard is test logic and must itself be verified to fire. Probe
  for a dependency the way the dependency actually presents (ask the tool
  where it is), never via a path assumption. Skips are loud — counted and
  reported — or the test fails on skip in gated contexts.
- One canonical aggregate gate (`just verify`: fmt-check + lint +
  arch-lint + full tests + generated-docs build) run before declaring any
  work done, **"so drift cannot survive even a missing hook."** Hooks are
  convenience; the recipe is the contract.
- Never `--no-verify`. The sanctioned bypass exists but demands a paper
  trail: "bypass with --no-verify *and file a bead explaining why*."
- When silently-accumulated debt is discovered: a dedicated Step 0 cleanup
  commit *before* the real work, never mixed into it.

## The three-tier latency topology

Gates are placed by cost, and the placement is documented in the hook
header itself:

1. **Commit (fast, blocking)**: fmt-check, clippy `-D warnings`,
   complexity gate on staged files, *direct* dependency-rule checks
   (cargo-metadata parsing — no compilation needed).
2. **Push (slow, blocking)**: *transitive* dependency-graph walk,
   regenerate generated docs/architecture site — "diagrams that cannot
   lie are diagrams the build proves can be rebuilt."
3. **Opt-in (heavy)**: mutation runs, full coverage × complexity reports,
   aggressive scanners — behind explicit recipes (`just metrics`,
   `just mutants`), never in hooks.

Advisory checks may ride in blocking hooks with `|| true` — they print,
they never block, and the split (BLOCKING vs ADVISORY) is declared at the
top of the hook so nobody has to infer it.

Missing-tool policy: a hook check whose tool isn't installed skips
silently — a contributor without the optional linter must not be unable
to commit. The canonical aggregate gate and CI are where a missing tool
is an error.

## Pick the cheapest honest mechanism

- Dependency/role rules: parse build metadata (`cargo metadata`) — exact,
  no build, no AST.
- Function identity and shape (change detection, generated docs): AST —
  the one place regex genuinely can't.
- Regex checks: advisory only. A regex gate that blocks will false-
  positive its way into being bypassed, which is worse than not having it
  (see vacuous green). Field example: the port-purity regex catches `use
  tokio` but not `tokio::fs::read(...)` — fine for an advisory nudge,
  dishonest as a blocker.

## A lint validates the consequence, not the premise

Mechanical architecture checks verify structural *consequences* (dep
edges), never semantic *premises* (whether the label is true). A crate
can self-declare `role = "domain"`, have zero deps, pass every gate — and
be an orchestrator. Two responses, use both:

- Audits re-read the code behind self-declared labels (orchestration,
  async combinators, port-driving inside a "domain" crate are use-case
  smells; "if it defines a port AND orchestrates over it, it's usecase").
- Prefer mechanisms where the wrong state is **uncompilable** over
  mechanisms where it is merely **labeled**: a no-default-method
  supertrait turns "wrong backend wired in" into a compile error; a
  metadata label turns it into a passing lint. "The role tag is
  decoration unless CI enforces it" — and even enforced, it only proves
  the consequence.

## Exception files (grandfathered debt) — self-emptying or lying

- **Stale exceptions fail the lint.** An exception that no longer matches
  a real violation is itself a gate failure — paid-off debt rots loud,
  and the file empties itself honestly.
- **New code never gets an exception.** Exceptions are for existing debt
  only; each entry carries consumer, dependency, ticket id, and reason —
  no anonymous suppressions.
- Paid-off entries survive as commented tombstones with their ticket ids:
  the file documents the debt's history after it's gone.
- Source-level suppressions follow the same regime: site-local
  `#[allow(lint, reason = "...")]`, never file-level blankets. The strict
  config earns its keep — deny-level lints caught a real lock-contention
  bug masked by a significant-Drop temporary.

## Gate failure UX — error messages that teach

Every blocking failure prints: the rule in one sentence ("all arrows
point inward"), the two legitimate fixes ("invert the dependency, or
change the role"), and the sanctioned bypass with its paper-trail
requirement. An agent hitting the gate cold must be able to act from the
message alone — a bare exit 1 trains agents to bypass.

Negative tool decisions are recorded **at the enforcement site**: when a
scanner is rejected from the hook, the hook comment names the rejection
and the concrete false positives that earned it — so the next session
doesn't re-add it, and the decision is re-litigatable on evidence.

## Stop hooks (turn-end gates for agents)

A Stop hook that blocks turn-end on a broken build keeps the workspace
always-buildable — and makes "hand the human a non-compiling hole" an
illegal move. Two corollaries:

- Deliberately-incomplete handoffs must still compile: `todo!()` bodies
  or stub types, never missing functions.
- Keep Stop-hook checks fast and deterministic: a compile-red handoff
  traps the session in a hook loop (the hook re-fires with the same
  error forever).
