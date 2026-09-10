---
id: "ticket-779-type-world-predicates"
title: "Dual-world rules are copied across Checker walks"
kind: ticket
status: promoted
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---

# Dual-world rules are copied across Checker walks

## Signal

Dragons audit 2026-09-10. One `Type` enum mixes JS values and native types. Dual-world branches repeat in `checker_expr.rs` (`as`, call, new, pointer store), `checker_ops.rs` (unary/binary), and `checker_assign.rs` (null → `*T`, boolean → native bool). `is_dual_world_boundary` exists on the type module and is not the only copy.

## Fit

Promoted to [[slice-786-type-world-predicates]].

## Notes

- `check_expr` is a 983-line match. Do not invent a new crate seam. Put world tests on `Type`.
- IR keeps `Type` on every expr. Backends need it. Do not hide the field.

## Parent

[[slice-786-type-world-predicates]]
