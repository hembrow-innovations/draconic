---
title: Three layers of plan
impact: CRITICAL
tags: [layers]
---

# Three layers of plan

## Intent (sticky)

Why this project exists, success looks like X, we will not do Y. One page at `.heio/planning/intent.md`. Change rarely, and only on purpose.

## Map (semi-sticky)

Roadmap of **locations**, optional `locations/<slug>.md` files, plus sprint groupings. `.heio/planning/roadmap.md`, `.heio/planning/locations/`, `.heio/planning/sprints/<id>/shape.md`. Add a location bullet without rewriting siblings. Grow a sub-map when a location needs depth.

## Work (fluid)

Task-pool files and incoming tickets. This layer is supposed to churn. Rigidity comes from writing tasks during the grill and treating that list as the plan. A planning sitting publishes the pool after freeze, in the same sitting. Tasks live in `.heio/planning/task-pool/`. Tickets stay in `.heio/tickets/` until triage promotes one.
