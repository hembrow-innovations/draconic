---
name: design-tree
description: Design tree interview. Counterpart and notebook are branches.
disable-model-invocation: true
---

# Design tree

Interview until you share an understanding. Map the work as a **design tree**. Every settled decision opens the decisions that hang off it. The **frontier** is every decision whose prerequisites are already settled.

This skill plans. Confirm, then Publish. Implementation is a later session.

Load **management** before any write under `.heio/`. Load **docs** before any write under `docs/`. Load **domain-modeling** when a term or ADR belongs in the vault.

## 1. Pick

Name the **counterpart**. The **notebook** follows. Say both in one line before the first round.

- **user** in this chat. Default.
- **user** in a file. Copy the management round template to `.heio/planning/rounds/rounds-<NN>-<slug>.md`. `sitting_kind: planning`. Status `awaiting-answers`. Keep the template headings. Append later rounds in that file. Never rewrite an earlier round.
- **product** peer. Read [references/counterpart-product.md](references/counterpart-product.md) before round one.
- **panel**. Read [references/counterpart-panel.md](references/counterpart-panel.md) before round one.
- The tree will not fit in one sitting. Read [references/wayfinder.md](references/wayfinder.md) and follow that file instead of steps 2–3.

Done when counterpart and notebook are named, or when wayfinder has taken over.

## 2. Frontier

Finding facts is your job. Dispatch a lookup for anything you can observe. A running lookup is an unsettled prerequisite. Ask the rest of the frontier now.

Ask the whole frontier in one **round**. Number each question. Give a recommended answer. A question that depends on another still open in this round waits for a later round.

Use a subagent to research the web to find real world examples of solutions to this question.

```markdown
❓ **Q1** - **<question title>**: <question body, might be multiple paragraphs, including multiple choices>

➡️ <recommended answer>

> <Real world example of how this problem was solved and why (1-2 sentence explanation). omit if none found>
```

Wait for the counterpart. Record the answers in the notebook. Append the next frontier.

Done when every open decision has a recorded answer, no lookup is in flight, and the counterpart has confirmed a shared understanding.

## Confirm

Stop. Summarize:

- Destination, if this sitting touched it
- Every in-slice Done + `EXPECT:`
- The tracer-bullet list: title, slice, `blocked_by`, AFK or HITL, what it delivers

Ask granularity, blockers, merge or split, HITL vs AFK. Prefer AFK. Each task is one sitting, vertical, sized for a fresh context window. Prefactoring is its own first task and blocks the rest.

Wait. Iterate the list until the user confirms the understanding and the breakdown.

Done when the user confirms both, or auto skipped confirm.

## Publish

Copy templates from **management**. Write the sitting's settled files in one pass. This session writes every slice and task file. Do not hand that off.

Load **to-slices** and **to-tasks**.

## Ticket

User names a ticket, or an inbound signal that is not yet a ticket.

1. If no file exists, copy `templates/ticket.md` into `.heio/planning/tickets/`.
2. Interview only enough to triage. The solution does not live on the ticket.
3. Same rule every time:
   - Fits an unblocked active slice → **TASK**. Status `promoted`. Write the task file (`ready`, mode, `blocked_by`) and the slice `[[id]]` link.
   - Fits the project, not this slice → **TICKET**. Status `parked`.
   - Would rewrite a location destination during a workflow → **ESCALATE**. Stop. The map needs a wayfinder sitting.

Done when the ticket has a status and a verdict.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Planning a slice is not VERIFY. VERIFY checks oracles on the slice file.
