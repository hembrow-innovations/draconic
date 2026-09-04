---
id: "s-d01-workspace-timeout"
title: "D01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:01:13Z"
updated_at: "2026-09-04T14:13:35Z"
claimed-by: a2c9cc68-3dfa-4447-8926-6de2d458adf4
---

# D01 workspace tests finish

## Why

Review of [[s-d01]] left ROADMAP D01 unfinished: O1–O3 (`release_binaries`, `install_script`, `install_smoke`) held, but O4 `cargo test --workspace` timed out at 120s. The distribution location still needs the D01 Loop to leave the workspace green, not only the install integration suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D01 work. The release-binaries, install-script, and install-smoke harnesses stay green. If the hang comes from the D01 change, fix that release-binaries + install-to-PATH surface so both the workspace check and those integration tests hold. Mark D01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d01]]**: that slice stays sealed `failed`
- **D01.01–D01.03**: host-triple artifact, install script, and fresh-PATH smoke children already `done`
- **D02**: Toolchain version pin in `draconic.toml`
- **D03**: Reproducible-build byte identity
- **D04**: Cross-compile matrix and CI jobs for non-host triples
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: workspace tests finish after the D01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test release_binaries --offline && cargo test -p draconic-integration-tests --test install_script --offline && cargo test -p draconic-integration-tests --test install_smoke --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4469c88e9e945000 bytes=99001 at=2026-09-04T14:13:06.614Z

- [x] O2: D01 release → install → fresh PATH stays locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test release_binaries
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8bc7d1788ed1e244 bytes=2874 at=2026-09-04T14:13:07.381Z

- [x] O3: D01 one-line install script places `draconic` on PATH
  CHECK: cargo test -p draconic-integration-tests --test install_script
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=69cb1f0de8b814ee bytes=3134 at=2026-09-04T14:13:07.871Z

- [x] O4: D01 install smoke: fresh PATH `draconic -V` / parse hello
  CHECK: cargo test -p draconic-integration-tests --test install_smoke
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=30529e471a73582b bytes=2923 at=2026-09-04T14:13:08.701Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d01-workspace-timeout]]`

## See also

ROADMAP.md D01, `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/install_script.rs`, `tests/integration/tests/install_smoke.rs`, `scripts/release-artifact.sh`, `scripts/install.sh`, `website/install.md`, CONTEXT.md, [[distribution]], [[s-d01]], [[ticket-126-d01-workspace-timeout]].
