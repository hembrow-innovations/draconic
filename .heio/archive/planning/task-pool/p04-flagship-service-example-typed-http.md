---
id: "p04-flagship-service-example-typed-http"
title: "P04 Flagship service example: typed HTTP + fs/config + git dep (after H17 + K09)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:52:41Z"
updated_at: "2026-09-05T14:12:35Z"
---

# P04 Flagship service example: typed HTTP + fs/config + git dep (after H17 + K09)

## Done

ROADMAP P04 is implemented test-first on both targets: an `examples/` Program uses typed HTTP, reads config via host fs, and depends on a git module path; integration tests lock native HTTP + fs/config + git-dep behavior; the js target builds and runs the portable config/git-dep path (HTTP listen stays native-first); P04 is `done`.

## Context

Roadmap ID **P04** (Flagship service example: typed HTTP + fs/config + git dep (after H17 + K09)). Product location: one in-repo Program that combines typed HTTP, filesystem config, and a git module dependency after H17 success programs and K09 package e2e. H17.01–H17.03 already land http-echo and todo native serve; K09.01–K09.02 already land temp git dep + consumer e2e. Integration harness `tests/integration/tests/flagship_service.rs`. Mark P04 `done` only when those tests are green. Not P01 fizzbuzz, P03 docs site, P05 shebang, H17.04 JS/Node bridge, K10 pkg-lib/pkg-consumer (already `done`), K11 post-v1 packaging, R02 permission model, or replacing `examples/http-echo` / `examples/todo`.

## Verify

`cargo test -p draconic-integration-tests --test flagship_service` prints `test result: ok.` Workspace `cargo test --workspace` stays green. P04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (P04), `examples/` (new flagship service; do not replace `examples/http-echo` or `examples/todo`), `tests/integration/tests/flagship_service.rs`, `tests/integration`, `tests/packages` as needed for the git-dep path

## Links

[[s-p04]] [[ticket-117-p04-flagship-service-example-typed-http]]

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-integration-tests --test flagship_service
- **result**: win
- **gap**: none
- **workspace**: cargo test --workspace → test result: ok.
