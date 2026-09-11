---
id: "ticket-808-frontend-check-source-script-only"
title: "Frontend check_source does not retry Module"
kind: ticket
status: open
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-11T21:44:22Z"
updated_at: "2026-09-11T21:44:22Z"
---

# Frontend check_source does not retry Module

## Signal

Dragons audit 2026-09-12. Frontend owns Script versus Module, then check, then lower. `parse_source` retries Module after Script (`crates/draconic-frontend/src/lib.rs:63`). `check_source` and `compile_source` parse Script only (`lib.rs:74`). Same buffer can parse and fail check. Closed [[ticket-780-frontend-cli-bypass]] wired CLI and test262 through Frontend. Leftover is split policy inside the facade.

## Fit

this project, later slice

## Notes

- No workspace test failed on this sitting.
- `load_program` (`lib.rs:116`) has its own Script-then-Module retry for paths.
- Do not mark E17.02 or E18.44 done.
