---
name: session-lifecycle
description: Use at the START of every working session, at the END of every working session, and immediately after any context compaction/summarization. Defines the orientation protocol (first steps), the handoff protocol (last steps), and how to exploit the fact that agent sessions start from a clean state.
---

# Session Lifecycle

You start every session with amnesia. This is not a defect to apologize for —
it is the design constraint the whole kit is built around, and done right it
is an advantage: you never carry yesterday's bad assumptions, half-remembered
hacks, or stale mental models. A human can't reset; you do, for free.

The corollary is absolute: **anything not written to the repo does not
exist.** Your "memory" is, in priority order:

1. `specs/` — WHAT the system must do (the contract)
2. `docs/adr/` — WHY it is shaped this way (the settled arguments)
3. The bead / task graph — WHAT work is open, in flight, blocked
4. `HANDOFF.md` — WHERE the last session stopped, exactly
5. Git history of green commits — HOW the work got here (replayable
   red/green/refactor sequence)
6. `proptest-regressions/` — every counterexample ever found
7. The check infrastructure — the rules, mechanically enforced

A past session that did work without updating these stole from you.
Do not be that session.

## First steps (orientation protocol — run before ANY code)

1. Read `CLAUDE.md`. The file is the rule; never trust a summary's
   paraphrase of it.
2. `git status` + `git log --oneline -10`. Dirty tree? Stop — a previous
   session violated handoff. Report it; do not silently absorb someone
   else's uncommitted mess into your work.
3. Read `HANDOFF.md` and the assigned bead. The bead tells you WHAT
   (it references REQ IDs); the spec files are the contract; the bead
   never prescribes HOW.
4. `./scripts/trace-audit.bb` — inherit a clean baseline or report a dirty
   one before it becomes yours.
5. `rg 'REQ-<ID>'` for the requirement you're picking up — read the spec,
   its anchor scenarios, its existing tests, and any linked ADRs.
6. **Run the baseline green.** Execute the test suite (or at minimum the
   harness for the area you're touching) and confirm it passes BEFORE
   writing anything. A red baseline is inherited breakage — stop and report;
   never start work on red, or your failures and the previous session's
   become indistinguishable.
7. **Read the template slice.** If the repo designates a reference slice
   (the walking skeleton: its spec, its tests, its production code), read
   all three and run its harness green. It is the worked example of every
   convention in this file — pattern-match it. When your output diverges
   from the template's shape, the template wins unless an ADR says otherwise.
8. State your plan in one or two lines: which REQ, which scenario/law is
   next, expected first RED. Then start the loop.

Total cost: a few minutes. Skipping it is how sessions re-solve solved
problems, contradict accepted ADRs, and duplicate half-finished work.

## Environment hazards

- Never invoke interactive or TUI commands — they block the session
  permanently. Always use the non-interactive/JSON flags (`--json`,
  `--robot`, `--no-pager`, `-y`); if a tool has no non-interactive mode,
  report it instead of running it.
- Pipe pagers away (`git --no-pager`, `| cat`) and never run watch modes.

## After context compaction (mid-session amnesia)

Compaction is a partial session reset that doesn't announce itself honestly:
you keep a lossy summary and lose the actual rules. Treat it as a session
start: re-run steps 1, 2, and 6 of orientation. Cheap insurance against the
classic failure where post-compaction "you" confidently violates a policy
that pre-compaction "you" had read.

## Last steps (handoff protocol — the session is not done until these are)

1. **Never end mid-red.** Either reach green and commit, or revert to the
   last green commit and record why in the handoff. A dirty tree or a red
   bar is a trap left for an amnesiac — i.e., for you.
2. Run the gates: tests, `check-guard.bb`, `trace-audit.bb`, and the
   quality gate if production code changed. (The Stop hook enforces this;
   don't make it fire.)
3. Update the bead: status, link to commits, REQ IDs touched.
4. Write `HANDOFF.md` (overwrite, don't append — it describes NOW):
   - Current REQ and which scenarios/laws are done vs remaining
   - The exact next test to write (this is the highest-value line in the
     file: it converts the next session's cold start into a warm one)
   - Surprises: anything learned that the spec/ADRs don't yet capture —
     and if it's load-bearing, don't just note it, file it (spec revision
     proposal or ADR draft). Route by kind: discovered WORK → bead;
     discovered CONSTRAINT on someone else's bead → warning block appended
     to that bead, marked verified-or-suspected (tracker-discipline);
     reusable seam / paid-for gotcha / discipline correction → memory
     (memory-discipline). A lesson that cost a red→green cycle and isn't
     written down will be paid for again next session.
   - Open questions awaiting the human
5. Commits use scoped style: `<scope>: <description>` where scope is the
   capability/module touched (`sale:`, `pricing/discount:`), REQ ID in the
   body. Check-infrastructure changes use the `check-change:` scope —
   human-approved, standalone, as per CLAUDE.md.

## The work loop (literal, per work item)

1. Query the tracker for ready work (non-interactive: `br ready --json`);
   claim exactly ONE bead → status in_progress. If high-value work sits
   "blocked", audit its deps before believing it: a dep pointing at a
   parent capability when the bead only consumes one closed child is a
   mislabeled dependency, not missing work — retarget it
   (tracker-discipline).
2. Implement it via the kit's order of work (spec → types → red/green/
   refactor → gates), edge cases per the parity/acceptance spec if one exists.
3. Verify with receipts: tests, clippy, fmt, gates — output shown, not
   asserted.
4. Close the bead with a close-reason that is a handoff report, not a
   "done": what shipped, decisions taken, follow-ups filed BY ID, and
   verification status. Before closing: anything you called "deferred" or
   "out of scope" has a bead — search the tracker or create one
   (tracker-discipline; this rule has no exceptions).
5. Sync tracker state, commit. The commit body carries the REQ ID(s); the
   commit message ends with the bead id — commit ↔ bead ↔ REQ is the
   traceability triangle, and all three corners are grep-able.
6. Back to 1, or hand off.

One bead, done properly with passing specs and a commit, beats ten
half-built ones. Do not try to do everything in one session — the
dependency graph will hand you the next piece. Over-claiming is the
session-level version of speculative code: work nothing forced you to start.

## Exploiting the clean state

- Write everything for a reader with zero context, because that reader is
  you, tomorrow. If a handoff note only makes sense with today's context,
  it's useless by definition.
- Ambient knowledge is a bug. If you find yourself relying on something you
  "know" that isn't in specs/ADRs/code, that knowledge dies at session end —
  capture it or lose it.
- Fresh eyes are real: orientation is also review. If the spec you're
  picking up doesn't make sense without the previous session's context,
  that's a spec defect — report it rather than reverse-engineering intent
  from code.
