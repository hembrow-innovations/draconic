---
name: heio-planning
description: One planning sitting. Grill until shared understanding, then publish frozen slices and a ready task-pool. Also triage a ticket.
disable-model-invocation: true
---

# Planning sitting

Grill until the frontier is empty. Confirm. Publish every settled slice and its task-pool files. Do not build.

Load **heio-stack** before any write under `.heio/`. Load **docs** before any write under `docs/`. Load **domain-modeling** when a term or ADR belongs in the vault.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Rounds

The frontier is every decision whose prerequisites are already settled. Ask the whole frontier in one round, at most 4 questions. Wait for the user's answers before the next round.

Talk to the user with `ask_user_question`. Do not dump questions in the transcript.

Each question is multiple choice:

- **2–4 options**: short label (1–5 words), plus what choosing it means
- **recommended first**: append `(Recommended)` to that label
- **one call**: do not stack `ask_user_question`

A custom answer is always available. Do not add an "Other" option. A question that depends on another still open in this round belongs later. Finding facts is your job. Decisions are the user's.

Work destination first when intent or locations are missing. Then this sprint's **tracer bullets**: each slice's Done, oracles, `blocked-by`, and whether each unit is **AFK** or **HITL**. Fog last.

A slice you cannot demo or learn from in one sitting is two slices. Split before you write.

Done with rounds when the frontier is empty.

## Confirm

Stop. Summarize:

- Destination, if this sitting touched it
- Every in-slice Done + `EXPECT:`
- The tracer-bullet list: title, slice, blocked-by, AFK or HITL, what it delivers

Ask granularity, blockers, merge or split, HITL vs AFK. Prefer AFK. Each task is one sitting, vertical, sized for a fresh context window. Prefactoring is its own first task and blocks the rest.

Wait. Iterate the list until the user confirms the understanding and the breakdown.

Done when the user confirms both.

## Publish

Copy templates from **heio-stack**. Write the sitting's settled files in one pass.

Sticky map, only when this sitting settled them:

- `.heio/planning/intent.md`
- `.heio/planning/roadmap.md`
- `.heio/planning/locations/<slug>.md` only when a bullet needs depth
- `.heio/planning/sprints/<id>/shape.md` listing the in-slices. Status `active` when every in-slice is frozen.

Every settled slice:

- `.heio/planning/sprints/<id>/slices/s-<slug>.md`
- Status `frozen`. `EXPECT:` frozen. First `CHECK:` drafted
- Pool section lists every task-pool `[[id]]` this slice owns

Every task on those slices:

- `.heio/planning/task-pool/<id>.md` from `templates/pool-task.md`
- Status `ready`
- `mode: afk` or `mode: hitl`
- `blocked-by` names gating task ids, or none
- **Done**, **Context** (current vs desired behavior, interfaces, out of scope), **Verify** with `scope:`

A slice still in fog stays off disk. A HITL task is `ready` with `mode: hitl`. Drain skips it until a human is present.

Done when every settled slice is `frozen` with `EXPECT:`, every linked task-pool file is `ready` with mode and blocked-by, and the slice Pool links those ids.

## Ticket

User names a ticket, or an inbound signal that is not yet a ticket.

1. If no file exists, copy `templates/ticket.md` into `.heio/tickets/`.
2. Interview only enough to triage. The solution does not live on the ticket.
3. Same rule every time:
   - Fits an unblocked active slice → **TASK**. Status `promoted`. Write the task-pool file (`ready`, mode, blocked-by) and the slice `[[id]]` link.
   - Fits the project, not this slice → **TICKET**. Status `parked`.
   - Would rewrite a location destination during a workflow → **ESCALATE**. Stop. The map needs a wayfinder sitting.

Done when the ticket has a status and a verdict.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Planning a slice is not VERIFY. VERIFY checks oracles on the slice file.
