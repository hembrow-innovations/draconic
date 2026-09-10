---
id: "ticket-780-frontend-cli-bypass"
title: "CLI and test262 re-wire stages around Frontend"
kind: ticket
status: closed
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T18:30:00Z"
---

# CLI and test262 re-wire stages around Frontend

## Signal

Dragons audit 2026-09-10. Frontend owns Script vs Module, then check, then lower. `cmd_parse` calls `parse_and_dump`. `extract.rs` calls `parse_module` only. REPL clones IR and splices Node `util.inspect` after `emit_js`. test262 force-link does `link_entry` then `check_module` then `lower`.

## Fit

Promoted to [[slice-787-frontend-cli-seam]]. Slice is `met`. Closed.

## Notes

- Build and check already use Frontend. Fmt uses `parse_source`.
- Native build re-reads source for `SourceDebug` instead of carrying it through Frontend.

## Parent

[[slice-787-frontend-cli-seam]]
