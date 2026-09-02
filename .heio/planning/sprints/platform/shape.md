---
id: "platform"
title: "platform"
kind: "sprint"
status: "active"
tags: []
created_at: "2026-09-02T12:00:00Z"
updated_at: "2026-09-02T13:03:20Z"
---

# platform

## Grouping

Location: remaining platform-capability and leftover conformance after the language spine. Pump mints one ROADMAP todo at a time; Plan adds the slice here.

## Slices in

- **s-e17-02**: E17.02 other non-strict legacy remainder
- **s-e17-02-workspace-timeout**: E17.02 remainder workspace tests finish
- **s-e17-02-remainder**: E17.02 other non-strict legacy remainder
- **s-d01**: D01 Release binaries + install script; one-line install to PATH
- **s-d02**: D02 Toolchain version pin in `draconic.toml`; CLI enforces or warns
- **s-d03**: D03 Reproducible builds: same source + pin → documented-equivalent artifacts
- **s-d03-01**: D03.01 Document reproducibility expectations (timestamps, paths)
- **s-d03-02**: D03.02 Same source + pin → byte-identical or documented-equivalent emit
- **s-d04**: D04 Cross-compile matrix: linux/darwin/windows × amd64/arm64 (as available)
- **s-d04-02**: D04.02 Matrix docs + CI jobs for available OS/arch pairs
- **s-d05**: D05 Binary size opts: strip / LTO flags documented and testable
- **s-d05-01**: D05.01 CLI/build flags: strip symbols
- **s-d05-02**: D05.02 LTO (or designed) flag documented; size delta smoke
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
- **s-e18-44**: E18.44 Untracked ECMA-262 remainder beyond E01–E18 children
- **s-s02**: S02 Expand Test262 allowlist / promote first failure cluster (see **E19.02**)
- **s-h00**: H00 Host I/O surface policy: module/global shape, error model, js hard-error vs polyfill matrix
- **s-h01**: H01 Process: args, env, exit
- **s-h02**: H02 Stdio: stdout / stderr / stdin
- **s-h03**: H03 Path helpers (string ops; no I/O)
- **s-h04**: H04 Filesystem: read / write / dirs
- **s-h05**: H05 Time, clock, timers (job-queue integrated)
- **s-h06**: H06 TCP sockets (sockets-first)
- **s-h07**: H07 Async socket I/O + job queue
- **s-h08**: H08 UDP
- **s-h09**: H09 DNS
- **s-h10**: H10 HTTP/1.1 thin helpers (plaintext) on sockets
- **s-h11**: H11 TLS
- **s-h12**: H12 WebSocket
- **s-h13**: H13 HTTP/2 (later; not v1 bar)
- **s-h14**: H14 Signals
- **s-h15**: H15 Subprocess
- **s-h16**: H16 OS misc
- **s-k01**: K01 Manifest (`draconic.toml`): module path, deps, optional path→git URL map
- **s-k02**: K02 Lockfile (draconic.lock): resolved pins
- **s-k03**: K03 Module cache: layout, git clone/fetch, checkout by OID
- **s-k04**: K04 Version resolve: semver tag → commit OID; fail closed
- **s-k05**: K05 CLI: `draconic get`, `draconic mod tidy`
- **s-k07**: K07 Build integration: auto-fetch; `--offline`
- **s-k08**: K08 Integrity: verify lock hashes; refuse tampered cache
- **s-k09**: K09 E2E: temp git dep + consumer Program
- **s-k11**: K11 Post-v1 packaging (not v1 bar)
- **s-k11-02**: K11.02 `replace` directive: fork/local override
- **s-k11-03**: K11.03 Multi-module monorepo (subdir module paths)
- **s-k11-04**: K11.04 Module proxy/mirror (git still canonical)
- **s-k11-05**: K11.05 Yank/retract when advisory source configured
- **s-f02**: F02 C callbacks: Draconic fn as extern C pointer; host invokes
- **s-f04**: F04 Link external static lib (`.a`); call one symbol
- **s-f07**: F07 Bindgen-ish: generate externs from C header subset
- **s-f08**: F08 Unsafe/native-only FFI diagnostics; JS hard-error; clear spans
- **s-c01**: C01 Worker / OS thread: spawn isolate running module/fn; join/terminate; no shared JS heap by default
- **s-c02**: C02 Message-passing channels: send/recv; structured-clone or transfer policy; bounded buffer as designed
- **s-c03**: C03 `once` / thread-safe init; mutex only if Runtime internals need it
- **s-c04**: C04 Parallel `draconic test`: multi-fixture workers; deterministic aggregate exit
- **s-c05**: C05 Structured cancellation / timeout helpers on async work (channels + timers)
- **s-c06**: C06 Optional later: shared-memory atomics (advanced; not v1 bar)
- **s-l01**: L01 Encoding: UTF-8 bytes↔string, Base64, hex
- **s-l02**: L02 Collections helpers (groupBy/chunk/Deque as designed; not redundant with Array/Map/Set)
- **s-l02-01**: L02.01 `groupBy` / `chunk` (or designed names) on arrays
- **s-l02-02**: L02.02 Deque (or designed): push/pop both ends
- **s-l03**: L03 Crypto: SHA-256 digest + secure random bytes
- **s-l04**: L04 Compression later: gzip/deflate byte buffers
- **s-l05**: L05 In-language test framework (`describe`/`it`/`expect` or designed) via `draconic test`
- **s-l06**: L06 Logging: leveled logger; stderr/stdout sink
- **s-l07**: L07 Flags/CLI parse: argv → typed options/positionals
- **s-l07-01**: L07.01 Parse long/short flags + positionals from string array
- **s-l07-02**: L07.02 Typed options (bool/string/number); help text as designed
- **s-l08**: L08 URL / query parse + serialize
- **s-l10-01**: L10.01 HMAC-SHA256 (after L03)
- **s-l09**: L09 MIME multipart later (HTTP-shaped programs; after H10)

## Slices out

- not a rewrite of ROADMAP.md itself
- not a new language spine cluster unless a ticket names it

## See also

intent.md, roadmap.md, ROADMAP.md.
