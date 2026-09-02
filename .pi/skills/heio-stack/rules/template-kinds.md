---
title: Kind to template to destination
impact: HIGH
tags: [template]
---

# Kind to template to destination

Copy the template. Do not invent a new skeleton.

- **intent**: `templates/intent.md` → `.heio/planning/intent.md`
- **roadmap**: `templates/roadmap.md` → `.heio/planning/roadmap.md`
- **location**: `templates/location.md` → `.heio/planning/locations/<slug>.md`
- **sprint**: `templates/sprint-shape.md` → `.heio/planning/sprints/<id>/shape.md`
- **slice**: `templates/slice.md` → `.heio/planning/sprints/<id>/slices/s-<slug>.md`
- **ticket**: `templates/ticket.md` → `.heio/tickets/ticket-<NN>-<slug>.md`
- **pool-task**: `templates/pool-task.md` → `.heio/planning/task-pool/<task>.md`
- **archive index**: `templates/archive-index.md` → `.heio/archive/index.md`

Shared fields: `templates/required-fields.md`.
