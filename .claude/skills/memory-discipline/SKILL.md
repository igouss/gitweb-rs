---
name: memory-discipline
description: Use when deciding whether something learned this session is worth persisting to agent memory, when writing or updating a memory file, and when a saved memory turns out to be wrong. Defines the four memory genres, the selection bar, the evidence-anchor rule, and correction discipline. The repo carries specs/ADRs/beads; memory carries what none of those can.
---

# Memory Discipline

Memory is not a diary. A 30-file memory corpus from a four-day port kept
~25 commits/day sustainable across amnesiac sessions because every file
passed a bar: **its absence already burned a red→green cycle, or would
burn one next session.** Files that don't pass that bar are noise that
dilutes recall.

## What memory is NOT for

Don't save what the repo already records: code structure (read the code),
past work (git log), open work (the tracker), requirements (specs/feature
files), settled decisions (ADRs). Memory holds the residue: empirical
facts, traps, seams, and discipline — the things a fresh session cannot
derive and will otherwise rediscover the expensive way.

## The four genres (each has a shape; match it)

1. **Discipline rules** — process corrections, usually learned from an
   incident. Shape: *the rule → "Why:" (the failure it prevents) → "How to
   apply:" (concrete commands/steps) → the incident that taught it.*
   These are the most transferable memories and the most rigidly
   structured. ("Stub wrong-but-valid, never todo!() — panicking stubs
   abort the suite at the first scenario instead of giving the full red
   picture.")
2. **Templates** — one canonical worked layout the next slice copies, with
   per-layer file paths and a DON'T-FORGET list for the steps that fail
   silently. One per repo; everything else links it.
3. **Seam catalogs** — per-closed-slice: status line with the commit hash →
   the reusable seams by path and signature → **imperative consumer
   notes** ("future X MUST drive through this, not a new method") →
   gotchas → follow-up bead ids. The seam memo converts past work into a
   binding API for future sessions; without it the next session reinvents
   the port method.
4. **Reference/gotcha facts** — empirically verified library/tool behavior
   not derivable from your code: where a library diverges from the
   reference tool, terminator/newline ownership conventions, feature-flag
   traps, resource-lifetime traps. Explicitly framed as cache: "saves
   re-reading the library source next slice."

## The evidence-anchor rule

Every claim carries its anchor, or it's vibes:

- Every **DONE** → a commit hash.
- Every **deferral** → a bead id.
- Every **empirical claim** → its verification method ("verified vs real
  git", "proven by reversing the actual output bytes", reference source
  file + line).

That triple — artifact anchor, tracker anchor, evidence anchor — is what
makes a memory corpus auditable instead of folklore.

## Correction discipline

Memories are falsifiable. When a saved claim is disproven:

- Amend **in place** with a loud `**CORRECTION (was wrong here):**` marker,
  preserving the wrong belief as a warning — a silent rewrite erases the
  trap for the next session that re-derives the same wrong belief.
- The same applies outward: when implementation disproves a bead's or
  spec's claim, record the correction there with the proof, don't just
  quietly do the right thing.
- Memories are living ledgers: update status (DONE, blocking child,
  follow-ups) as work lands; a stale "pending" is a lie with a timestamp.

## Index hygiene

- The index (MEMORY.md) is a routing table, not a summary: one line per
  file, and that line is the single most operational fact compressed to a
  clause. The frontmatter `description` is load-bearing — recall decisions
  are made on it, so write it as the compressed rule itself.
- **Verify links.** Even a disciplined corpus shipped a dangling index
  entry pointing at a file that was never written. Treat the index as
  derived: after writing a memory, check every `[[link]]` and index line
  resolves to a real file. Mechanical: `scripts/memory-lint.bb
  <memory-dir>` checks dangling wiki-links, dangling/missing index
  entries, unindexed files, frontmatter, and oversize — run against the
  field corpus it found two dangling links nobody had spotted.
- **Split files that accrete.** A single-topic gotcha file grew to 28KB;
  past ~10KB recall pulls in mostly-irrelevant bulk. Section it or split
  by sub-topic before it gets there.

## Style

- Emphasis is a finite resource: CAPS only for the trap words (MUST, NOT,
  WRONG, CORRECTION, DON'T FORGET) so a skim catches exactly the traps.
- Write for the reader with zero context, because that reader is you,
  next session.
