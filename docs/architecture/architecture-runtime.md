---
id: "architecture-runtime"
title: "Runtime"
kind: architecture
description: "Native Runtime linked into binaries: GC heap, job queue, Promise ABI, host I/O, JS polyfills, abort policy."
domain: draconic
area: runtime
tags: [runtime]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Runtime

## Overview

The Runtime is the native support linked into binaries produced by the [[architecture-backend-llvm|LLVM backend]]. It is not a bytecode VM. JS values live on a tracing GC heap. Native types stay unboxed and off that heap. Async work runs on a FIFO job queue. Host I/O is a C substrate (sockets first, then thin HTTP helpers). The JS backend does not link this C Runtime; it consumes JS polyfills exported from the same crate. Glossary: [[CONTEXT]]. Heap vs unboxed layout: [[architecture-dual-worlds]] and [[system-design-gc-runtime]]. Eval compile path: [[architecture-embed]]. Pipeline and CLI: [[architecture-pipeline]], [[architecture-cli]].

## Context

Native Programs need JavaScript object semantics (identity, cycles, closures) and unboxed systems types in one process. [[0003-gc-runtime-and-dual-worlds]] rejects ownership-only and arena-only heaps because they cannot host a full ECMA-262 superset. [[0008-host-io-sockets-first-http]] places host networking under Roadmap **H**: TCP listen/accept/connect/read/write first, then plaintext HTTP/1.1 helpers on those sockets. [[0011-catchable-exceptions-vs-abort]] splits language `try`/`catch` from process abort.

The crate `draconic-runtime` owns:

- **C translation units**: `draconic_rt.c` / `draconic_rt.h` (GC, jobs, Promises, timers, abort) and `draconic_rt_host.c` / `draconic_rt_host.h` (host I/O substrate).
- **Rust ABI catalog**: `abi.rs` — LLVM-text declare/call shapes that match those C symbols.
- **JS polyfills**: strings returned from `lib.rs` modules for the [[architecture-backend-js|JS backend]].
- **Fuzz hook**: `fuzz.rs` (`fuzz_runtime`, Roadmap **R05.02**).

`lib.rs` still comments “embed later”; Embed is a sibling crate ([[architecture-embed]]), not a C symbol in this Runtime.

## Design

### Cores in `lib.rs`

`lib.rs` re-exports the public modules, embeds the C sources for tests and tooling, and builds `libdraconic_rt.a` (`build_runtime_static_lib`). Public modules:

- **`abi.rs`**: `AbiFn` plus every `draconic_rt_*` / `draconic_rt_host_*` symbol the LLVM backend declares. Also host error codes (`HOST_OK` … `HOST_E_ADDR`), `host_error_name`, and JS polyfills for process, stdio, path, fs, time, workers, channels, cancel tokens.
- **`collections.rs`**: `collections_js_polyfill` — `groupBy` / `chunk` / `Deque` (**L02**).
- **`compression.rs`**: `compression_js_polyfill` — gzip/deflate via Node `zlib` when present (**L04**).
- **`flags.rs`**: argv parse (`parse_flags`, `parse_flags_typed`) and `parse_flags_js_polyfill` (**L07**).
- **`host_js_bridge.rs`**: `http_js_polyfill`, `dns_js_polyfill`, `tcp_js_polyfill` (**H17.04** Node/portable bridges).
- **`logging.rs`**: `create_logger_js_polyfill` (**L06**).
- **`mime.rs`**: multipart parse/serialize plus `mime_js_polyfill` (**L09**).
- **`crypto`** (inline in `lib.rs`): SHA-256, OS CSPRNG, HMAC-SHA256, AES-256-GCM polyfills (**L03** / **L10**). Native `fill_random` reads `/dev/urandom`.
- **`testing`** (inline): `describe_it_js_polyfill` — `describe` / `it` / `expect` / hooks (**L05**).
- **`url`** (inline): `parse_url` / `parse_query` / `serialize_query` and matching polyfills (**L08**).

`print_hello` writes the Runtime hello line. `apply_runtime_link_flags` adds pthread and, on macOS, Security plus CoreFoundation for TLS (**H11**).

### GC, jobs, Promises

See [[system-design-gc-runtime]] for the heap. The C ABI in `draconic_rt.h`:

- **GC**: `draconic_rt_gc_init` / `shutdown`, `alloc_string` / `alloc_object`, root push/pop, `collect`, `live_count`, alloc threshold (**N09.05**), alloc budget (**R01.02**).
- **Job queue** (**N06.01**): `job_enqueue`, `job_drain`, `job_pending`. Drain is FIFO. Nested enqueue during drain runs after the current job. Re-entrant drain is a no-op. After microtasks, drain promotes due timers, signal watches, async process waits, and host IO readiness, sleeping instead of busy-spinning (**H05.05**, **H07.01**, **H14**, **H15.03**).
- **Timers** (**H05**): `timer_set` / `timer_set_interval` / `timer_clear` share one id space; due timers become jobs at the end of each drain wave.
- **Promise ABI** (**N06.02–N06.10**): `promise_new`, construct with executor, resolve/reject, `then`, `finally`, `all` / `race` / `allSettled` / `any`, `promise_await`. Reactions enqueue jobs. Arrays and objects on the same GC heap hold opaque `void*` slots (GC pointers or inttoptr numbers).

### Host I/O substrate

Host I/O is not Dual-worlds scalars and not ECMA-262. Locked shape: [[0008-host-io-sockets-first-http]].

`draconic_rt_host.h` / `draconic_rt_host.c` implement:

- **Handles and errors**: opaque `DraconicHostHandle`; codes `OK`, `INVAL`, `NOENT`, `NOSYS`, `BADF`, `EXIST`, `PERM`, `IO`, `NOMEM`, `AGAIN`, `CONN`, `ADDR`.
- **Bytes boundary** (**H00.03**): `DraconicHostBytes` views model `ArrayBuffer` / `Uint8Array` (embedded NUL is payload). Covered by `host_bytes_tests.rs`.
- **Process / env / exit / pid** (**H01**), **stdio** (**H02**), **path** (**H03**), **fs** including open/read/write/seek (**H04**), **time** (**H05**).
- **TCP first** (**H06**): listen (port 0 → ephemeral), accept, connect, read/write, shutdown. Async accept/connect/read/write return Promises (**H07**).
- **UDP** (**H08**), **DNS** (**H09**, IPv4), **HTTP/1.1** parse/write on sockets plus chunked TE (**H10**), static-file one-shot (`http_serve_static`, **H17.03**).
- **TLS wrap** of an existing TCP handle (**H11**), **WebSocket** handshake/frames (**H12**), **HTTP/2** thin preface + single-stream helpers (**H13.01** — not a full multiplexed stack).
- **Signals**, **subprocess**, **cwd/os/temp/home**, **workers**, **channels**, **once**, **internal mutex**, **cancel tokens**, **shared-memory atomics**. Tests: `host_abi_tests.rs`, `host_worker_tests.rs`, `host_once_tests.rs`, `host_mutex_tests.rs`, `host_cancel_tests.rs`, `host_atomics_tests.rs`.

JS target: unsupported host APIs hard-error until an explicit bridge row. **H17.04** bridges HTTP (portable), DNS, and sync TCP via Node in `host_js_bridge.rs`.

### Default permission policy

Do not treat v1 as Deno lockdown. [[CONTEXT]] **Default permission policy** and [[0008-host-io-sockets-first-http]]:

- **Unset or empty `DRACONIC_PERMISSIONS`**: permissive. Filesystem read/write and TCP listen/connect succeed on surfaces that already exist (**R02.04**).
- **Non-empty grant subset**: comma tokens `fs-read`, `fs-write`, `net-listen`, `net-connect`. Missing grant → `EPERM` (**R02.01**, **R02.02**).
- **CLI grants are R02.03**: `draconic run --allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` / `--allow-net-connect` install that env on the child. No flags means the env is left unset (permissive).

### Catchable exceptions vs abort

[[0011-catchable-exceptions-vs-abort]]:

- **Catchable (R04.01)**: user `throw` of a JS value; ECMA `TypeError` / `RangeError` / `ReferenceError` and friends. `try`/`catch` runs. Uncaught still exits non-zero. On the JS backend these are ordinary throws. Native E10 observations for throw/try/catch are folded at emit in the LLVM backend — there is no C exception unwind ABI in this Runtime.
- **Abort (R04.02)**: `draconic_rt_abort` prints `draconic_rt: abort`, a backtrace (**R06**), then `abort()`. Root-stack underflow/grow failure, job-enqueue OOM, and similar invariant sites call the same die path. These never become JS values. Tests: `abort_policy_tests.rs`.
- **Resource budgets (R01)**: GC alloc over budget returns `NULL`; eval time budget sets an exceeded flag. Fail closed at the C ABI, not a catchable JS exception. Tests: `gc_alloc_budget_tests.rs`, `eval_time_budget_tests.rs`, `r01_resource_limits_tests.rs`.

Host ABI failures on native print a token (`ENOENT`, `EPERM`, …) and exit 1. On JS they throw a catchable `Error` with `.name === "HostError"` and `.code` equal to that token.

### JS polyfills exported for the JS backend

Where they exist, the Runtime crate exports `*_js_polyfill()` strings (not the C heap): collections, compression, crypto, flags, logging, mime, URL/query, testing, process/env/exit/pid, cwd/chdir, hostname/os, temp/home, subprocess, workers, channels, cancel, clocks/timers, stdio, path, fs, plus `host_js_bridge` HTTP/DNS/TCP.

### Fuzz

`fuzz.rs` exports `fuzz_runtime`. Byte input is lossily decoded and run through URL, query, flags, and MIME parse. Ok and Err are discarded; panic is failure (**R05.02**). Embed fuzz is [[architecture-embed]], not this hook.

## Trade-offs

Tracing GC plus unboxed natives matches Dual worlds ([[0003-gc-runtime-and-dual-worlds]]) at the cost of a C heap, root stack, and abort-on-invariant. Sockets-first host I/O ([[0008-host-io-sockets-first-http]]) keeps HTTP a helper, not a Node-shaped `http` module as the only entry. Permissive default permissions match already-working host surfaces; opt-in grants are additive, not a locked-down default. Abort vs catchable ([[0011-catchable-exceptions-vs-abort]]) keeps OOM and GC faults out of `try`/`catch`.

## Consequences

LLVM emit must declare Runtime symbols from `abi.rs` and link `libdraconic_rt.a`. JS emit must inject polyfills rather than calling C. Embed eval limits share the fail-closed policy with GC budgets. Remaining eval completeness (dynamic source compiled inside a running native process) is [[architecture-embed]] / [[0004-full-ecma-262-and-embed]], not a hidden C `eval` symbol. HTTP/2 and WebSocket helpers are the Roadmap **H13** / **H12** subsets already in `draconic_rt_host.c`; they are not a full browser networking stack.
