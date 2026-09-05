---
id: "l07-workspace-timeout"
title: "L07 workspace tests finish"
kind: task
status: ready
mode: afk
blocked-by: ["d04-workspace-tests"]
tags: []
created_at: "2026-09-05T00:54:44Z"
updated_at: "2026-09-05T05:46:30Z"
---

# L07 workspace tests finish

## Blocked by

[[d04-workspace-tests]]: live successor of archived [[d04-workspace-disabled-gha]] / [[s-d04-workspace-disabled-gha]]. `cargo test --workspace --offline` finishes (does not hang) and fails in `-p draconic-integration-tests --test cross_compile` (`docs_ci_and_host_llvm_emit_form_one_available_matrix` missing live `.github/workflows/release-artifact.yml` after `97bbcc4`). L07 flags are not the cause. stdlib_flags stays green. Do not start until D04 workspace tests pass. [[s-d04-workspace-tests]] / [[ticket-187-workspace-disabled-gha-workflow]].

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L07 work; the stdlib flags conformance tests stay green.

## Context

Roadmap ID **L07** (`Flags/CLI parse: argv → typed options/positionals`). Review of [[s-l07]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_flags`) stayed green. The stdlib location still needs the L07 Loop to leave the workspace green, not only the argv → typed options/positionals fixtures. If the hang comes from the L07 change, fix that flags/CLI parse surface so the workspace check and those fixtures hold. Mark L07 `done` only when those tests are green. Not L07.01 long/short flags + positionals, L07.02 typed options / help text, H01 process argv, L05 test framework, L06 logging, Node `util.parseArgs` identity, or a full GNU getopt clone. Do not re-open [[s-l07]].

## Verify

`cargo test --workspace` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_flags` still prints `test result: ok.` L07 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L07), `tests/conformance/tests/stdlib_flags.rs`, `tests/conformance/fixtures/stdlib/flags`, flags/CLI parse surface as needed to unhang workspace tests after L07

## Links

[[s-l07-workspace-timeout]] [[ticket-186-l07-workspace-timeout]] [[s-l07]]
