---
id: "slice-216-workspace-test-budget"
title: "Workspace tests finish inside the oracle budget"
kind: slice
status: active
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T04:39:34Z"
updated_at: "2026-09-06T18:00:00Z"
---

# Workspace tests finish inside the oracle budget

## Why

Reviewer misses on the D04 / L07.02 / L09 / L10 / L10.02 / P04 / R03 / R03.02 / R04 / R05 / R05.02 / R06 workspace oracles. `cargo test --workspace` printed `test result: ok.` but blew the 600s CHECK budget, or exited 101 while the substring was present from passing crates. Language rows stay `done`. This cut beats the workspace gate, not a new Loop atom.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. `release_artifact` stays green against `.yml.disabled`.

## Blocked by

None.

## Non-goals

- **Minting new Loop atoms** for D04 / L07.02 / L09 / L10 / L10.02 / P04 / R03 / R03.02 / R04 / R05 / R05.02 / R06
- **Restoring live `.yml` names**
- **Marking E17.02 or E18.44 done**

## Oracle checklist

- [ ] O1: workspace tests finish inside the CHECK budget
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: D01.01 release-artifact workflow reader stays locked
  CHECK: cargo test -p draconic-integration-tests --test release_artifact
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- `[[task-217-workspace-test-budget]]`

## See also

[[ticket-204-d04-workspace-tests-timeout]] [[ticket-205-l07-02-workspace-tests]] [[ticket-206-l09-workspace-timeout]] [[ticket-207-l10-02-workspace-tests]] [[ticket-208-l10-workspace-timeout]] [[ticket-209-p04-workspace-tests]] [[ticket-210-r03-02-workspace-tests]] [[ticket-211-r03-workspace-timeout]] [[ticket-212-r04-workspace-timeout]] [[ticket-213-r05-02-workspace-tests]] [[ticket-214-r05-workspace-timeout]] [[ticket-215-r06-workspace-timeout]]
