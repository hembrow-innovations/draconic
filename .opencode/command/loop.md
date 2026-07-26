---
description: Run the unattended draconic opencode-loop (default 100×) with stall watchdog
---

Run the unattended multi-iteration driver and keep it healthy until it finishes.

## What to do

1. Start (do **not** block the whole session waiting silently — stream / poll):

```bash
node .loop/opencode-loop.mjs ${1:-100}
```

Optional env (pass through if the user set them in `$ARGUMENTS`):

- `SLEEP=<seconds>` — pause between loops
- `STALL_SEC=<seconds>` — kill a hung iteration after this many seconds with no stdout (script default **600**)
- `STALL_ACTION=continue|abort` — after a stall, go to the next loop or stop (default **continue**)

If `$ARGUMENTS` is a number, use it as the loop count. If it includes flags/env-style tokens, honor them.

Examples the user might mean:

- `/loop` → 100 iterations
- `/loop 20` → 20 iterations
- `/loop 50 SLEEP=30 STALL_SEC=900`

2. **Stall watch (required):** the script already kills children that produce no stdout for `STALL_SEC`. You still must:
   - Confirm the process is actually running (pid, recent output).
   - If the **parent** `.loop/opencode-loop.mjs` itself stops printing for longer than `STALL_SEC` (or ~10 minutes if unset) with loops still remaining, treat that as a stall: inspect, restart the remaining count, or kill and report.
   - If a child is clearly wedged (no JSON events, no CPU, same loop header forever past the limit), kill it and continue / restart remaining loops.
   - Never sit idle “waiting to see” without checking process state.

3. When finished, report: loops completed, stall kills, non-zero exits, and whether ROADMAP progressed.

## Do not

- Do not run the draconic-loop skill yourself in this session — the child `opencode run` processes do that.
- Do not start a second full 100× driver on top of one already running unless the first is dead.
