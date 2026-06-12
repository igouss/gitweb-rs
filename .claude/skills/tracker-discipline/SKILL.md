---
name: tracker-discipline
description: Use when creating, decomposing, claiming, closing, or blocking on a work item (bead) — and whenever the words "deferred", "out of scope", or "follow-up" are about to leave your mouth. Defines bead anatomy for agent consumers, the standing meta-beads, follow-up capture, dependency hygiene, and the tracker as an inter-session message bus.
---

# Tracker Discipline (beads)

The bead graph is the single source of remaining work. If it isn't a bead,
it won't get done — a deferral living only in a close-reason, code comment,
or your head has no tracker home and rots silently. Field evidence: a
92-bead port closed 74 of them in four days with near-zero re-litigation,
because the tracker carried not just the work but the *conversation between
sessions*.

## Bead anatomy (what a bead an agent can implement looks like)

What-not-how, but "what" done properly is dense. The recurring sections,
in order:

1. **WHAT** — one paragraph, outcome-stated. No implementation steps.
2. **Reference anchor** — where the spec source lives, precise enough to
   re-check: spec/REQ id, or for oracle work the reference file + function
   + line range ("read git_commitdiff, ~7854-7857").
3. **WHY SEPARATE** — why this is its own bead and what it deliberately
   excludes. Every carve-out states this or it will be re-merged by a
   session that can't see the seam.
4. **Constraints/gating** — feature toggles, error behavior, which
   verification tier (byte-exact vs behavioral) applies.
5. **SEAMS** — the existing code seams to extend, by path and symbol.
   This is the anti-NIH payload: the next session reuses or reinvents
   depending entirely on whether this section exists.
6. **Acceptance edges** — the enumerated edge cases, tied to the parity/
   acceptance spec, plus the test-layer expectations.
7. **Ruled-out dead ends, with dates** — "verified 2026-06: no crate does
   X byte-exact; library Y has no encoder". Negative results are the most
   expensive knowledge to regenerate; date them so a future session knows
   when the survey went stale.
8. **Cross-links footer** — the conventions bead and the
   definition-of-done bead (below).

Two sections a second field project proved out, worth adding when the
work has design risk:

- **Risk, with the mitigation pre-planned** — not "this is risky" but the
  staged plan: "extract as a helper function first (no trait), prove both
  call sites, THEN introduce the trait only if a third caller appears."
- **DO NOT — the negative space.** Anti-over-abstraction guards written
  into the bead: "do NOT abstract for a hypothetical third caller; two
  adapters, one shared body — that is the actual shape." The cheapest
  place to stop speculative generality is before the session starts.

Code sketches in a bead are **illustrative, not binding**. The
implementer may deviate — and must argue the deviation in the commit
body ("heeded the bead's Risk guidance: helper functions, NOT a
strategy trait; a closed shape in one place beats an open trait").

A ~100-character bead is a backlog *stub*, not implementable work. Stubs
are fine as markers; enrich to the anatomy above before pickup, never
implement from a stub.

## Bead text loses to the structure

A bead that names a target location is making a structural claim, and
structural claims get verified against the actual dependency graph —
**twice**: by the author at writing time, and by the implementer at
start time. When they conflict, the structural rule beats the bead text;
override and document the deviation in the commit body, then correct the
bead. Field cost of skipping the author-side check: a placement the
dep-matrix lint forbade, discovered mid-epic, paid for with an unplanned
"slice 1.5" relocation commit. Two specific checks before writing a
placement into a bead:

- Does the named crate's role permit the dependencies this code needs?
- Do the types riding the new port's signature belong to an outer layer?
  A port whose argument bundle carries an outer-layer type must live in
  the outer layer — placement is decided by what types ride the
  signature, not by what the port is "about".

## Two standing meta-beads (write once, link everywhere)

- **The READ-FIRST conventions bead** — priority 0, deliberately never
  closed. Carries the non-negotiables (architecture, test layers,
  red→green discipline, banned dependencies) and the **core-innovation
  tripwires** — the exact `file:field`s that encode the project's
  reason-for-existing and must never flip, each cross-linked to the gate
  that enforces it (quality-gates). An amnesiac reads this bead before it
  can regress the one thing that matters; every other bead links it
  instead of repeating it.
- **The definition-of-done bead** — defines what "done/parity/accepted"
  *means*: the edge-case matrix, the verification oracle, the tiers.
  Acceptance sections in other beads point here.

This is how per-bead descriptions stay lean without losing context: shared
context has exactly one home.

## Decomposition (lazy, at pickup time)

- Initial planning creates coarse capability beads. Decompose into
  children **when the capability is picked up**, not upfront — by then you
  know the real seams. One action/endpoint/behavior = one child = full
  test layers.
- The decomposition itself is committed (tracker-state commit with a
  rationale body) and the parent's description accretes per-child DONE
  notes with commit hashes — the parent becomes the capability's ledger.
- **Mid-flight rescoping is normal, not failure.** When a child turns out
  to be two things, split again ("rescoped to the shippable half, other
  half split to a new bead") rather than letting it bloat.
- Close the epic when its last child closes — this requires an explicit
  habit; epics left open with all children closed are the most common rot.

## Follow-up capture (the deferral rule)

When you say "deferred", "out of scope", or "follow-up": **stop**. Either
(a) confirm an existing bead covers it — by *searching the tracker*, never
by assuming — or (b) create one, with full anatomy and correct deps,
**before closing the current bead**. Reference the new bead's id in the
close-reason. No exceptions; this rule is the difference between a tracker
and a diary.

## The tracker as an inter-session message bus

Three field-proven mechanisms:

- **Warnings appended to a FUTURE bead's description.** Discovered a
  constraint that lands on someone else's bead? Append a
  `FOUNDATION WARNING (found while doing X)` block to *that* bead: the
  discovered constraint, an explicit "do NOT blindly attempt Y", the
  enumerated options (a)/(b) with the decision left to the implementer,
  and a pointer to any proven reusable technique.
  **Mark each claim VERIFIED (with evidence) or SUSPECTED.** An unverified
  warning recorded as fact once drove a wrong out-of-scope decision that
  cost a correction commit, doc rewrites, and two new beads.
- **Close-reasons are handoff reports**, not "done". Name what shipped
  (modules, decisions taken), follow-ups filed by id, and verification
  status ("tests/clippy/fmt green; commit pending"). For multi-slice
  beads, the full landing report: the commit list with one-line
  summaries, the acceptance criteria re-verified at close — including
  which residual grep hits *remain* and why each is legitimately out of
  scope — side artifacts named ("new crate X"), and any
  fixed-inline-instead-of-filed choices justified with a pointer to the
  commit that argues them.
- **Decisions are revisable, and the correction is itself recorded.** When
  a bead's claim is disproven, correct it in writing with the evidence —
  never silently do the right thing while the bead still asserts the wrong
  one. Owner rulings get recorded too ("owner confirmed byte-exact is the
  bar; the shortcut is rejected; the prerequisite stands").

## Dependency hygiene

- **Before treating a bead as blocked, audit its deps.** The classic false
  blocker: the dep points at a *parent* capability when the bead only
  consumes one already-closed *child*. Read what the bead actually needs,
  retarget the dep to the specific sub-bead, verify with `br ready --json`.
  A mislabeled dependency hiding ready work is the highest-value thing a
  triage pass can find.
- Cross-cutting concerns (error mapping, input validation, cache headers,
  filtering) are their own beads that action beads depend on or reference —
  one shared rule is one bead, never N copies.
- Use the typed edges (`blocks`, `parent-child`, `discovered-from`) over
  prose provenance where the tracker supports them — prose works but is
  unqueryable. In practice `discovered-from` is the one everyone forgets.

## Mechanical audit

`scripts/tracker-audit.bb [issues.jsonl]` checks the JSONL mirror
directly (no tracker CLI needed): open epics whose children are all
closed, closed deferrals whose close-reason cites no bead id (accepts
short id suffixes — that's how close reasons actually cite), dangling
dependency edges, and still-open stub beads. The deferral check has a
known false-positive mode ("deferred to the render adapter" — placement,
not work); triage by hand, don't tune it away.

## Tracker-state commits

Every bead mutation is its own scoped commit (`beads:` scope): a one-line
JSONL diff with a full rationale body. The tracker's own history becomes
an auditable decision log — who decided what, when, and why, replayable
with `git log`. Never bundle tracker mutations into feature commits.
