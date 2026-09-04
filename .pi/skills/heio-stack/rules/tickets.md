---
title: Tickets
impact: HIGH
tags: [tickets]
---

# Tickets

A ticket is an inbound **product** signal. Bug, complaint, request, idea. It is not automatically work. It is not map hygiene.

Copy `templates/ticket.md`. Place it at `.heio/tickets/ticket-<NN>-<slug>.md`. Status starts `open`.

## Triage, same every time

- Fits an unblocked active slice → **TASK**, now. Status `promoted`. Write a task-pool file (`ready`, mode, blocked-by) and link it from the slice.
- Fits the project, not this slice → **TICKET**, park. Status `parked`. Pull into a later slice.
- Would rewrite a location destination → **ESCALATE**. Status stays `open` until the map changes.

## Rot

At sprint close, every leftover `open` or `parked` ticket is either re-filed onto the next sprint or set `dropped` with one line why. Closed and dropped tickets move to `.heio/archive/tickets/`. Tickets are not a second brain.
