---
title: Kind to template to destination
impact: HIGH
tags: [template]
---

# Kind to template to destination

Copy the template. Do not invent a new skeleton.

- **intent**: `templates/intent.md` → `.heio/planning/intent.md`
- **roadmap**: `templates/roadmap.md` → `.heio/planning/roadmap.md`
- **sprint**: `templates/sprint-shape.md` → `.heio/planning/sprints/<id>/shape.md`
- **slice spec**: `templates/slice-spec.md` → `.heio/planning/sprints/<id>/slices/s-<slug>/spec.md`
- **slice oracles**: `templates/slice-oracles.md` → `.../s-<slug>/oracles.md`
- **slice tasks**: `templates/slice-tasks.md` → `.../s-<slug>/tasks.md`
- **ticket**: `templates/ticket.md` → `.heio/tickets/ticket-<NN>-<slug>.md`

Shared fields: `templates/required-fields.md`.
