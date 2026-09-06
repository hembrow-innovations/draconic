---
id: "ticket-139-f02-workspace-timeout"
title: "F02 workspace tests did not finish (O2 timeout)"
kind: ticket
status: closed
ticket_type: feature
labels: feature
tags: []
sprint: "platform"
slice: "slice-270-f02-workspace-timeout"
created_at: "2026-09-04T15:13:19Z"
updated_at: "2026-09-06T18:00:00Z"
---

# F02 workspace tests did not finish (O2 timeout)

## Signal

slice-271-f02 Review O2 (`cargo test --workspace`) timed out at 120s. O1 ffi_callback was met. ABANDON leftover after --reverify; home for the timeout remainder.

## Fit

This project, later slice. Caused by slice-271-f02 failed oracle O2. Not L02.01.

## Notes

- **caused-by**: slice-271-f02
- **failed oracle**: O2
- **CHECK**: cargo test --workspace
- **EXPECT**: test result: ok.
- **EVIDENCE**: unmet exit=timeout match=yes bytes=56838 at=2026-09-04T15:11:55.620Z
- **Roadmap ID**: F02
