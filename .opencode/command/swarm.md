---
description: One Roadmap swarm wave (default serial wave=10); runs in a subagent
subtask: true
---

Run **one swarm wave** only. Do not implement Roadmap items yourself — spawn the driver and report.

## Args

`$ARGUMENTS` may include:

- `parallel` or `serial` (default **serial** — safe on one worktree)
- `wave=N` (default **10**)
- env-style: `SLEEP=30` `STALL_SEC=900` `STALL_ACTION=continue|abort`

Examples:

- `/swarm` → serial wave of 10
- `/swarm parallel wave=10`
- `/swarm wave=5 STALL_SEC=900`

## What to do

1. Parse `$ARGUMENTS` for mode + wave size + env tokens. Export any `KEY=val` env tokens for the child process.
2. Start (stream / poll — do not sit idle):

```bash
node .loop/opencode-swarm.mjs $ARGUMENTS
```

If `$ARGUMENTS` is empty, run:

```bash
node .loop/opencode-swarm.mjs wave=10
```

3. When the process exits, report briefly:
   - exit code
   - slots run / stalls / errors
   - ROADMAP `todo` before→after (run `node .loop/roadmap-status.mjs`)
   - whether the board is empty

## Do not

- Do not run the draconic-loop skill in this session — child `opencode run` processes do that.
- Do not start a second swarm while one is still running.
- Do not loop waves here — that is `/orchestrate`.
