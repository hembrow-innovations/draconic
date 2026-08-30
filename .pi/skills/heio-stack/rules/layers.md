---
title: Three layers of plan
impact: CRITICAL
tags: [layers]
---

# Three layers of plan

## Intent (sticky)

Why this project exists, success looks like X, we will not do Y. One page at `.heio/planning/intent.md`. Change rarely, and only on purpose. Human or a **heio-wayfinder** pass writes it. Builder agents read it. They leave it untouched.

## Shape (semi-sticky)

Roadmap of sprints, plus the current sprint's slices. `.heio/planning/roadmap.md` and `.heio/planning/sprints/<id>/shape.md`. You can swap *how*, drop a slice, split one, add one — as long as the sprint still means something. Planner pass. Frozen before tasks exist.

## Work (fluid)

Tasks and incoming tickets. This layer is supposed to churn. Rigidity comes from writing tasks too early and treating that list as the plan. Only the active slice has `tasks.md`. Tickets stay in `.heio/tickets/` until triage promotes one.

Intent + roadmap: human (or a planner pass) only. Builder agents read, never edit.
