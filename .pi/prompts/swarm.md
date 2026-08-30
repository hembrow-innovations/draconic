---
description: One Roadmap swarm wave (default serial wave=10)
argument-hint: "[parallel|serial] [wave=N]"
---

Run **one swarm wave** only. Do not implement Roadmap items yourself — spawn the driver and report.

## Args

`$ARGUMENTS` may include:

- `parallel` or `serial` (default **serial** — main worktree only)
- `wave=N` (default **10**)
- env-style: `SLEEP=30` `STALL_SEC=900` `STALL_ACTION=continue|abort`

**Parallel worktrees:** each slot gets `.loop/worktrees/<name>` + branch `swarm/<name>`. After the slot finishes (ok / error / stall), the driver **always** removes that worktree and deletes the branch. Start/end/signal also run a full sweep so nothing dangles.

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
   - slots run / stalls / errors / merge failures
   - ROADMAP `todo` before→after (run `node .loop/roadmap-status.mjs`)
   - whether the board is empty
4. Confirm no dangling trees: `node .loop/worktree.mjs list` should show **no** `[swarm]` entries. If any remain, run `node .loop/worktree.mjs cleanup`.

## Do not

- Do not run the draconic-loop skill in this session — child `pi --print` processes do that.
- Do not start a second swarm while one is still running.
- Do not loop waves here — that is `/orchestrate`.
- Do not leave `.loop/worktrees/*` around — cleanup is mandatory.
