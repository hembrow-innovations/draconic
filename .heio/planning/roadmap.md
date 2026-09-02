---
id: "roadmap"
title: "Roadmap"
kind: "roadmap"
status: "active"
tags: []
created_at: "2026-09-02T12:00:00Z"
updated_at: "2026-09-02T12:00:00Z"
---

# Roadmap

Locations. Destinations, not a schedule. Language row status stays on dest ROADMAP.md.

## Locations

- **conformance**: leftover ECMA-262 rows and Test262 allowlist expansion are honest
  - bet: drain E17.02 / E18.44 / S02 before inventing new spine clusters
- **host-io**: host/process control, stdio, sockets, HTTP helpers on native
  - bet: sockets-first + HTTP helpers (ADR-0008)
- **packages**: Go-style git packages and draconic.toml
  - bet: git module paths, no central registry (ADR-0009)
- **ffi**: call out and in without leaving the language for routine C ABI work
  - bet: explicit boundaries; no silent JS/native confusion
- **concurrency**: workers / isolates a team can ship simple services with
  - bet: one spawn model, then channels; pivot if Runtime queue is the real seam
- **stdlib**: libs a service needs without leaving Draconic
  - bet: small honest surfaces over a Node-shaped kitchen sink
- **distribution**: shippable artifacts and install
  - bet: CLI + native binary + JS emit are enough to start
- **runtime-hardening**: reliability and security rows on ROADMAP R*
  - bet: catchable exceptions vs abort stay distinct (ADR-0011)
- **product**: public Learn/Reference and example programs
  - bet: docs site already exists; remaining P/S rows are polish

## See also

CONTEXT.md, ROADMAP.md, docs/adr/.
