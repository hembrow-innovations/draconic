---
id: "ticket-926-replace-relative-clone"
title: "Relative [replace] paths parse then fail at clone"
kind: ticket
status: open
ticket_type: bug
tags: [packages]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# Relative [replace] paths parse then fail at clone

## Signal

`[replace]` accepts `{ path = "../vendor/lib" }` and string `../…`. Clone then fails because git URL validation allows only absolute local paths. In-tree library iteration needs an absolute `--url` or `[urls]` git checkout.

## Fit

Unknown until triage. Schema vs fetch mismatch. Fits [[location-220-packages]].

## Notes

- `replace.rs` allows `./` and `../`.
- `validate_clone_url` in `crates/draconic-pkg/src/cache/cache_fetch.rs` and `validate_git_url` in `validate.rs` allow https, ssh, `file://`, or an absolute path.
- Get and tidy both clone through that.
- Git and module replace do flow through `resolve_git_url`. There is no CLI test for replace.
- `examples/pkg-consumer/README.md` works only with an absolute `--url`.

## Parent

[[location-220-packages]]

## What to build

A relative replace path either clones from the consumer workspace or is rejected at parse with a diagnostic that names the absolute-path rule.

## Blocked by

none
