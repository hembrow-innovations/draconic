---
description: AFK-style audit of exactly one ROADMAP.md row — run its tests, score JS/native/types claims, file tickets for gaps, then exit. Does not implement.
agent: build
argument-hint: "[Roadmap ID | continue | status]"
---

Load **roadmap-audit** and **management**. Run one sitting, then exit.

Arguments: $ARGUMENTS

- Empty or `continue` — next unaudited row in `ROADMAP.md` document order (ledger under `.heio/planning/rounds/`).
- A Roadmap ID — that row.
- `status` — progress only. No tests. No tickets.

This is not `/afk-task` and not **draconic-loop**. Do not implement. Do not edit `ROADMAP.md`. Do not start a second row. File gaps as tickets so a later sprint can promote them into slices and tasks.
