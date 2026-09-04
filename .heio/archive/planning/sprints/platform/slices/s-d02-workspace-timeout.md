---
id: "s-d02-workspace-timeout"
title: "D02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:06:07Z"
updated_at: "2026-09-04T14:15:27Z"
claimed-by: a562e975-4a17-433e-bf41-badead4a9dd6
---

# D02 workspace tests finish

## Why

Review of [[s-d02]] left ROADMAP D02 unfinished: O1–O2 (`toolchain_pin` on `draconic-cli` and `draconic-integration-tests`) held, but O3 `cargo test --workspace` timed out at 120s. The distribution location still needs the D02 Loop to leave the workspace green, not only the toolchain-pin suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D02 work. The CLI and integration `toolchain_pin` harnesses stay green. If the hang comes from the D02 change, fix that toolchain version pin in `draconic.toml` (CLI enforces or warns) so both the workspace check and those pin tests hold. Mark D02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d02]]**: that slice stays sealed `failed`
- **D02.01–D02.02**: manifest field and CLI mismatch path already `done`
- **D01**: Release binaries + install script
- **D03**: Reproducible-build byte identity
- **D04**: Cross-compile matrix and CI jobs
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: workspace tests finish after the D02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --test toolchain_pin --offline && cargo test -p draconic-integration-tests --test toolchain_pin --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=7b6a2ef23ff16174 bytes=96249 at=2026-09-04T14:15:05.821Z

- [x] O2: D02 CLI toolchain pin enforce/warn stays locked by `draconic-cli` tests
  CHECK: cargo test -p draconic-cli --test toolchain_pin
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c882caa94c8acc8f bytes=3161 at=2026-09-04T14:15:06.225Z

- [x] O3: D02 toolchain pin in `draconic.toml` stays locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test toolchain_pin
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=357d750d3d395d05 bytes=3018 at=2026-09-04T14:15:06.287Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d02-workspace-timeout]]`

## See also

ROADMAP.md D02, `crates/draconic-cli/tests/toolchain_pin.rs`, `tests/integration/tests/toolchain_pin.rs`, `crates/draconic-cli/src/toolchain_pin.rs`, `crates/draconic-pkg/src/toolchain.rs`, CONTEXT.md, [[distribution]], [[s-d02]], [[ticket-127-d02-workspace-timeout]].
