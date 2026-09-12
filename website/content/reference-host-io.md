---
title: host I/O
section: reference
status: shipped
---

# host I/O

Host I/O is how a Program talks to the machine. This page is lookup. The designed lesson is [host I/O](host-io.html).

Host APIs are free identifiers, not ESM imports.

## stdoutWrite

Save this as `hello-host.drac`. It builds today:

```drac
stdoutWrite("hello from Draconic\n");
```

```
draconic parse hello-host.drac
draconic check hello-host.drac
draconic run hello-host.drac
```

## Names

- Process and stdio: `stdoutWrite`, `stderrWrite`
- Filesystem: `readFileText`, `writeFileText`
- TCP sockets: `tcpListen`, `tcpAccept`, `tcpConnect`, `tcpRead`, `tcpWrite`, `closeTcp`
- HTTP/1.1 helpers on those sockets: `httpParseRequest`, `httpWriteResponse` — not a Node-shaped http module as the only entry

Listen and server paths started native-first. Those names also build on the JS backend through an explicit bridge. Native-only host APIs still hard-error on js.

Default permission policy is permissive; `--allow-*` on `draconic run` installs an opt-in grant subset.

v1 HTTP is plaintext HTTP/1.1. TLS, HTTP/2, and WebSocket are later.

Command flags: [CLI](cli.html).
