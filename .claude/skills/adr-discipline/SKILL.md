---
name: adr-discipline
description: Use before proposing any architectural, modelling, or dependency decision; after making one; or when tempted to restructure existing code whose shape seems wrong. ADRs are the memory that stops agents from relitigating settled decisions across sessions.
---

# ADR Discipline

ADRs exist because agents have no memory between sessions. Without them, every
session re-argues f64-vs-newtype for money, re-proposes the framework that was
already rejected, and "improves" structure that is load-bearing for reasons
not visible in the code.

## Before proposing anything structural

```bash
rg -il '<topic keywords>' docs/adr/
```

- **Accepted ADR covers it** → work within it. If you believe it is now wrong,
  surface the ADR to the human with your reasoning. Do not implement against
  it while waiting. Superseding an ADR is the human's call.
- **No ADR, but the existing code has a consistent shape** → the shape is the
  decision; it just wasn't recorded. Follow it, and offer to backfill an ADR.
- **No ADR, genuinely new decision** → make it, then record it (below) in the
  same piece of work.

## What gets an ADR (and what doesn't)

Yes: port/adapter boundaries, persistence and serialization choices, domain
model shape (the money/percent newtype decision), dependency adoption or
rejection, error-handling strategy, anything a future session might plausibly
"helpfully" undo.

The sharpest case of "would be helpfully undone" is the **core-innovation
decision** — the one choice that is the project's reason for existing. That
gets an ADR *and* a mechanical tripwire gate that fails the build if the value
flips (quality-gates): the ADR records why, the gate stops the silent revert an
amnesiac would otherwise ship. ADR and tripwire are two halves of the same
protection — argument and enforcement.

No: naming of one function, formatting, anything cargo fmt/clippy already
enforces, decisions fully expressed by a requirement in `specs/` (link the
spec instead).

## Writing one

Use `templates/adr.md`. File: `docs/adr/NNNN-short-title.md`, NNNN sequential.
Keep it under a page. The two sections that matter most:

- **Alternatives rejected, and why** — this is the anti-relitigation payload.
  An ADR that omits the rejected option will not stop a future session from
  proposing exactly that option.
- **Consequences** — including the bad ones you are accepting. Honest
  consequences are what let a future human judge whether the context changed.

Statuses: `proposed` → `accepted` → (`superseded by NNNN`). Never edit an
accepted ADR's decision; supersede it with a new one.

## Lightweight mode: decision records in commit bodies

Field-validated alternative for decisions scoped to one work item: the
decision lives in the body of a dedicated commit — often a one-line
tracker-state diff whose entire value IS the body ("Owner confirmed
byte-exact parity is the bar; the shortcut is rejected, so X stays a hard
prerequisite of Y"). Corrections of earlier decisions get the same
treatment, naming what was wrong and filing the follow-ups. Written *to
the next agent*, not to a changelog.

The rule that keeps this honest: a commit-body decision must be reachable
without archaeology — linked from the bead that carries the work. The
moment a decision constrains *future structure* (a boundary, a dependency,
a model shape — anything a later session might "helpfully" undo), it
graduates to a real ADR file; `git log` is not where anyone checks before
restructuring. Scoped-to-one-bead → commit body + bead link. Load-bearing
beyond the bead → ADR.

When a bead deliberately delegates a choice to the implementer ("engine
choice is yours"), the implementer records the decision with its why and
its application rule — that's an ADR (or a commit-body record + memory if
narrowly scoped), not a silent pick.

## Linking

ADRs may cite REQ IDs (a decision made to satisfy a requirement). Specs may
cite ADRs under `## Related ADRs`. Code cites neither — structure and types
should make the decision legible; the ADR explains why.
