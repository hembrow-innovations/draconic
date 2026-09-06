---
title: host I/O
section: learn
status: not-yet
---

# host I/O

Host I/O is how a Program talks to the machine: process, stdio, filesystem, sockets, then thin HTTP.

Default permission policy is permissive. A Program with no explicit grant subset may read and write the filesystem and listen and connect TCP on the targets that already expose those APIs. This is not a Deno deny-by-default. Opt-in `--allow-*` flags on `draconic run` install a grant subset when you want one. See [CLI](cli.html).

Networking is sockets-first. TCP listen, accept, connect, read, and write land on native. HTTP/1.1 helpers sit on those sockets. The designed surface is not a Node-shaped `http` module as the only entry. Listen and server paths are native first. The JS backend hard-errors unsupported host APIs until an explicit bridge exists.

v1 HTTP is plaintext HTTP/1.1. TLS, HTTP/2, and WebSocket are later.

The public site generator itself is a native Program that reads and writes files.

Continue to [packages](packages.html). Lookup: [host I/O](reference-host-io.html).
