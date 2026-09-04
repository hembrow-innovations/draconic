---
id: "platform"
title: "platform"
kind: "sprint"
status: "active"
tags: []
created_at: "2026-09-02T12:00:00Z"
updated_at: "2026-09-04T20:50:41.788Z"
---

# platform

## Grouping

Location: remaining platform-capability and leftover conformance after the language spine. Pump mints one ROADMAP todo at a time; Plan adds the slice here.

## Slices in

- **s-r01**: R01 Embed/eval resource limits: max source size, alloc/time budget
- **s-r02**: R02 Permission model (optional Deno-like): grant/deny fs and net; clear deny diagnostics
- **s-r02-01**: R02.01 Permission grants: fs read/write, net listen/connect (as designed)
- **s-r02-02**: R02.02 Deny path: clear diagnostic when host op lacks grant
- **s-r02-03**: R02.03 CLI/runtime flags to grant subset (opt-in permissions)
- **s-r02-04**: R02.04 Default policy documented (permissive vs locked-down as designed)
- **s-r03**: R03 Supply-chain policy tests once K08 lands (lock verify refuse tamper)
- **s-r03-01**: R03.01 Integration: tampered cache refused (depends K08)
- **s-r03-02**: R03.02 Integration: lock hash mismatch hard-fails build
- **s-r04**: R04 Panic/abort vs catchable exception policy; fixtures per class
- **s-r05**: R05 Fuzz/stress hooks: parser/embed/runtime entry points
- **s-r05-02**: R05.02 Embed/runtime fuzz or stress hooks
- **s-r06**: R06 Panic backtraces with source locations (ties U07 DWARF)
- **s-p04**: P04 Flagship service example: typed HTTP + fs/config + git dep (after H17 + K09)
- **s-p05**: P05 Shebang support docs + `#!/usr/bin/env draconic` run path (with **U14**)
- **s-s02**: S02 Expand Test262 allowlist / promote first failure cluster (see **E19.02**)
- **s-l05-workspace-timeout**: L05 workspace tests finish
- **s-l06**: L06 Logging: leveled logger; stderr/stdout sink
- **s-l07**: L07 Flags/CLI parse: argv → typed options/positionals
- **s-l07-01**: L07.01 Parse long/short flags + positionals from string array
- **s-l07-02**: L07.02 Typed options (bool/string/number); help text as designed
- **s-l08**: L08 URL / query parse + serialize
- **s-l09**: L09 MIME multipart later (HTTP-shaped programs; after H10)
- **s-l10**: L10 Crypto later: HMAC + AEAD (after L03)
- **s-l10-01**: L10.01 HMAC-SHA256 (after L03)
- **s-l10-02**: L10.02 AEAD encrypt/decrypt (after L03; algorithm as designed)

## Slices out

- not a rewrite of ROADMAP.md itself
- not a new language spine cluster unless a ticket names it

## See also

intent.md, roadmap.md, ROADMAP.md.
