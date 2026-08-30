---
title: Roadmap and sprints
impact: HIGH
tags: [sprints]
---

# Roadmap and sprints

## Roadmap

Bets over time. Not a schedule. “We are going here, in this order, for these reasons.”

Each line names a sprint and the decision that sprint must force. Dates are fine as *force functions*, bad as forecasts. Keep the sentence “this sprint exists to decide X,” not “ship by Friday.”

Human or **heio-wayfinder** writes `.heio/planning/roadmap.md`. Builder agents read it.

## Sprint

A date or event that forces a cut. Ship, review, kill, or re-scope. A destination *and* a decision point.

`.heio/planning/sprints/<id>/shape.md` lists slices in, slices out, and what this sprint means. Planner pass. Status `shaping` until the slice specs and `EXPECT:` lines exist, then `active`.

A sprint holds 2–4+ vertical slices. Only one slice is `active` at a time.

## End of sprint

Status `review`, then:

1. Keep, cut, or rewrite the next slices on the roadmap.
2. Re-file or drop leftover tickets (`rules/tickets.md`).
3. Status `closed`.
