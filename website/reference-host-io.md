---
title: host I/O
section: reference
status: not-yet
---

# host I/O

Host I/O is how a Program talks to the machine. This page is lookup. The designed lesson is [host I/O](host-io.html).

- Process, stdio, env, and filesystem
- TCP sockets: listen, accept, connect, read, write — native first
- HTTP/1.1 helpers on those sockets, not a Node-shaped http module as the only entry
- Listen and server paths are native first
- The JS backend hard-errors unsupported host APIs until an explicit bridge exists
- Default permission policy is permissive; `--allow-*` on `draconic run` installs an opt-in grant subset

v1 HTTP is plaintext HTTP/1.1. TLS, HTTP/2, and WebSocket are later.

The public site generator is a native Program that reads and writes files. Command flags: [CLI](cli.html).
