---
title: Force the loop
impact: CRITICAL
tags: [loop]
---

# Force the loop

Every output from this stack is one of four. Pick one. End the turn with it.

- **TASK**: it fits the active slice. Do it now, or write it onto that slice's `tasks.md`.
- **TICKET**: it belongs to the project, not this slice. File it under `.heio/tickets/` and leave the slice alone.
- **ESCALATE**: it changes the bet. Stop. Bump it to sprint or roadmap with the human.
- **VERIFY**: run the slice oracles. `--reverify` until `ALL MET`, or `ABANDON:` with a named home.

```
VERDICT: TASK | TICKET | ESCALATE | VERIFY
EVIDENCE: <one line>
```

Scope creep is changing the bet without saying so. Adding a task that still serves the slice is cheap. Saying "not this slice" is also cheap. Both staying cheap is the system working.

If a change cannot wait for the next slice, you are either in an incident, or the slice was too big.
