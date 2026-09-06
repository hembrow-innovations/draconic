---
description: Autonomously complete exactly one ready unit — a ready task from .heio/planning/tasks (TDD, in-scope only), then exit
agent: build
---

You are an autonomous agent completing exactly ONE ready unit from `.heio/planning/tasks`, then exiting.
A **ready unit** is a **task** under `.heio/planning/tasks`
(`status: ready` + `kind: task`) , carrying a `## Agent Brief` with scope/verification/
acceptance/oracles).

Arguments: $ARGUMENTS

- If a task path/name is given, use it (a `ready` task).
- Otherwise pick the lowest-numbered ready unit across `.heio/planning/tasks/*.md`
  (`status: ready`), none in `.heio/archive` - one shared id sequence, so lowest id wins regardless of kind.

Workflow (follow AGENTS.md and project conventions throughout):

1. Read the unit's file (the task). **Vault pack (step 0):** load skill **vault-pack** and run `pnpm vault:pack -- --unit <path-to-unit.md>`; **Read every Must-read path in full** (intent ladder, agent-gotchas, purpose + contracts for `area`). Skim Related only if needed. Do not freestyle-grep half the vault. Then any relevant slice under `.heio/planning/sprints/<sprint_name>/slice-<NN>-<slug>.md`. Broad code exploration → subagent summary only.
2. Claim the unit: a task → `status: claimed`.
3. Implement the unit 100% — TDD, no stubs, no skipped scope. Respect its scope (a task's "Scope (may touch)" list, or the ticket's `## Agent Brief`); do not make repo-wide changes. Behaviour work must **name contract promise ids** from the pack; never invent product rules. UI must use the ui packages: 
	- React Native packages: `ui-components-native`/`ui-infra-native`, 
	- Web packages: `ui-components-web`/`ui-infra-web`, 
	- design tokens.
4. Verify: run the checks the task lists (jest/vitest/playwright-cli/typecheck/biome as applicable) until green. 
5. Spawn ONE subagent to adversarially review your diff for defects and convention violations; require it to check **diff vs named promise ids** (and purpose out-of-scope) when behaviour changed; fix what it confirms.
6. Update docs: tick the matching checkboxes; add change to changelog. Then close by kind (use **obsidian-axi** `mv` so links survive), git commit work:
   - **task** → set `status: completed`, move to `.heio/archive/planning/tasks/`.
7. If genuinely blocked (missing decision, broken precondition), append a "## Blocked" section explaining exactly why and what's needed, revert the claim (task → `status: ready`), commit that, and exit — do NOT mark it completed/closed.

Rules: never touch other ticket/task files except this unit's own (and, for a task, its source slice/ticket); never run vault-wide or store-wide fixes. Any issues arise, create a ticket for reviewing.
