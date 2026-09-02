---
id: "k02-lockfile-draconic-lock-resolved-pins"
title: "K02 Lockfile (draconic.lock): resolved pins"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:34:28Z"
updated_at: "2026-09-02T13:34:28Z"
---

# K02 Lockfile (draconic.lock): resolved pins

## Done

ROADMAP K02 is implemented test-first on the compiler target: a Program's package graph pins resolved deps in `draconic.lock` (path, version, git URL, commit OID, tree SHA-256), parse/write reject malformed locks, and unchanged rewrite is byte-identical with packages sorted by path; `draconic-pkg` lock tests are green and K02 is `done`.

## Context

Roadmap ID **K02** (Lockfile (`draconic.lock`): resolved pins). K02.01–K02.03 already land lock entries (path + version + git URL + commit OID + content hash SHA-256), parse/write with reject-malformed, and stable serialize (sorted paths; byte-identical rewrite when unchanged); this sitting unifies them as one honest `draconic.lock` resolved-pins surface on the compiler target. Tests in `crates/draconic-pkg`. Mark K02 `done` only when those tests are green. Not K02.01–K02.03 as separate atoms, K01, K03, K04, or K08.

## Verify

`cargo test -p draconic-pkg lock` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K02), `crates/draconic-pkg`, `crates/draconic-pkg/src/lock.rs`

## Links

[[s-k02]] [[ticket-51-k02-lockfile-draconic-lock-resolved-pins]]
