---
id: "p05-shebang-support-docs-usr-bin"
title: "P05 Shebang support docs + `#!/usr/bin/env draconic` run path (with **U14**)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:30:20Z"
updated_at: "2026-09-02T22:30:20Z"
---

# P05 Shebang support docs + `#!/usr/bin/env draconic` run path (with **U14**)

## Done

ROADMAP P05 is implemented test-first on the compiler target: docs name `#!/usr/bin/env draconic`, an `examples/` Program starts with that shebang and runs via the U14 path, `crates/draconic-cli` tests lock that documented run path against the example, and P05 is `done`.

## Context

Roadmap ID **P05** (Shebang support docs + `#!/usr/bin/env draconic` run path (with **U14**)). Product location: documented shebang execution as a product path. U14 already lands `draconic run` and shebang-friendly invoke; this sitting adds the documented run path plus an example a stranger can chmod and execute. Tests under `crates/draconic-cli` (oracle harness `--test shebang`). Mark P05 `done` only when those tests are green. Not U14 itself, P01 fizzbuzz, P03 docs site, P04 flagship service, D01 release binaries, or changing the U14 default target policy.

## Verify

`cargo test -p draconic-cli --test shebang` prints `test result: ok.` Workspace `cargo test --workspace` stays green. P05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (P05), `crates/draconic-cli`, `crates/draconic-cli/tests/shebang.rs`, `crates/draconic-cli/tests/run.rs`, `examples/`, `website/cli.md`, `README.md`

## Links

[[s-p05]] [[ticket-118-p05-shebang-support-docs-usr-bin]]
