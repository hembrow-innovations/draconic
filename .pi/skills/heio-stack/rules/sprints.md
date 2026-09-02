---
title: Roadmap and sprints
impact: HIGH
tags: [sprints]
---

# Roadmap and sprints

## Roadmap

Locations. Not a schedule. Each bullet is a destination: this is working when. Add bullets. Do not rewrite siblings to add one.

A **bet** is an optional sub-bullet under a location: `bet: try X; pivot if Y`. If it wins, it becomes a location or a sprint grouping.

When a location needs depth, copy `templates/location.md` to `planning/locations/<slug>.md`. That file is the same shape: short why, nested location bullets, optional bets, enough links. No nested folders.

## Sprint

A grouping of slices. Named after a **location** or a **timebox** (`week-1`).

`.heio/planning/sprints/<id>/shape.md` lists slices in, slices out, and what this grouping is. Status `shaping` until the slice files have Done and `EXPECT:` lines, then `active`.

A sprint holds 2–4+ vertical slices. Many slices may be `active`. A slice that must wait names `blocked-by`.

## End of sprint

Status `review`, then:

1. Keep, cut, or rewrite the next slices. Grow or drop location bullets on the roadmap.
2. Re-file or drop leftover tickets (`rules/tickets.md`).
3. Move the sprint folder to `.heio/archive/planning/sprints/<id>/`. Add a one-liner to `archive/index.md`. Status `closed`.
