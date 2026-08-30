---
title: Tickets
impact: HIGH
tags: [tickets]
---

# Tickets

A ticket is an inbound signal. Bug, complaint, request, idea. It is not automatically work.

Any agent may create a ticket. Only triage (human or **heio-triage**) may promote one to a task.

Copy `templates/ticket.md`. Place it at `.heio/tickets/ticket-<NN>-<slug>.md`. Status starts `open`.

## Triage, same every time

- Fits the current slice → **TASK**, now. Status `promoted`. Write the task onto the active slice's `tasks.md`.
- Fits the project, not this slice → **TICKET**, park. Status `parked`. Pull into a later slice.
- Changes the bet → **ESCALATE**. Status stays `open` until the human rewrites sprint or roadmap. Do not sneak it in.

## Rot

Agents generate tickets cheaply. At sprint close, every leftover `open` or `parked` ticket is either re-filed onto the next sprint or set `dropped` with one line why. Tickets are not a second brain.
