---
title: Slices
impact: HIGH
tags: [slices]
---

# Slices

A slice is a vertical cut that is usable or learnable on its own. Not a layer (“backend first”). Outcome-shaped. It hangs off a **sprint**, which groups work for a location or a timebox.

A slice is **one markdown file**: status, oracle checklist, durable links to task-pool ids. Path: `.heio/planning/sprints/<id>/slices/s-<slug>.md`. Copy `templates/slice.md`. There is no slice folder and no `spec.md`, `oracles.md`, or `tasks.md`. Oracles and task-pool links live on the slice file. Sprint `shape.md` stays the grouping.

A slice you cannot demo or learn from in one sitting is two slices. If a change cannot wait, the slice was too big.

## Shape the slice

Write done in words on the slice file. Then immediately write oracles. If you cannot write a `CHECK:` / `EXPECT:`, the done is still mush. Stop and sharpen.

Name `blocked-by` when this slice must wait on another. Unblocked slices may run in parallel.

Carry enough ADRs, specs, and paths that a stranger does not hunt.

- **shaping**: Done and `EXPECT:` are still forming.
- **frozen**: Done and `EXPECT:` exist. Work may hang off the slice as task-pool files.
- **active**: linked task-pool work is in progress. Many slices may be `active`.

## Close

Oracles hold and every linked task-pool id is `completed` → status `met`. Links are never dropped. Then move the slice with its sprint, or leave it until sprint archive.

Or every leftover oracle has `ABANDON:` with a named next artifact (ticket id or “drop from sprint”), status `abandoned`. Abandoned is a handoff back to planning. It is not a green checkbox.
