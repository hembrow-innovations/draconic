# Panel

The counterpart is a **panel**, not the user. Each frontier is one **round**. The round file is the notebook.

Load **management** before any write under `.heio/`. Spawn OpenCode subagents. Do not load Pi plugins.

## Personas

- **architect** — types, signatures, module boundaries
- **product** — customer, wedge, scope to cut
- **coder** — cost, testability, reversibility

Use a dest agent only when that name is executable. Otherwise a fresh subagent with the persona in the task.

**round-orchestrator** may spawn. Panelists and **judge** do not.

## Notebook

One round at `.heio/planning/rounds/rounds-<NN>-<slug>.md`. `sitting_kind: planning`.

Search `.heio/planning/rounds/` and `.heio/archive/planning/rounds/` for an existing sitting on this topic. Update that file if you find one.

## Round

Read [round-contracts.md](round-contracts.md) before spawning **round-orchestrator**.

If that name is not executable, the parent runs the contracts itself.

Wait for the round to return before the next frontier.

Done when every round in the file matches the contracts, the frontier is empty, and the user has confirmed a shared understanding. Then return to Confirm on the parent skill.

On publish, load **to-slices** and **to-tasks** from the selected answers. The round file stays the interview.
