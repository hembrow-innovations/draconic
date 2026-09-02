---
title: Force the loop
impact: CRITICAL
tags: [loop]
---

# Force the loop

Every output from this stack is one of four. Pick one. End the turn with it.

- **TASK**: it fits an unblocked active slice. Do it now, or add a task-pool file and link it from the slice.
- **TICKET**: it belongs to the project, not this slice. File it under `.heio/tickets/` and leave the slice alone.
- **ESCALATE**: the change would rewrite a location destination. Stop. Bump it to the map.
- **VERIFY**: check the oracles on the slice file until they hold, or `ABANDON:` with a named home.

```
VERDICT: TASK | TICKET | ESCALATE | VERIFY
EVIDENCE: <one line>
```

Scope creep is rewriting a location without saying so. Adding a task that still serves the slice is cheap. Saying "not this slice" is also cheap. Both staying cheap is the system working.

If a change cannot wait for the next slice, you are either in an incident, or the slice was too big.
