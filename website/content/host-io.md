---
title: host I/O
section: learn
status: shipped
---

# host I/O

Host I/O is how a Program talks to the machine: process, stdio, filesystem, sockets, then thin HTTP.

Host APIs are free identifiers on the global object. They are not ESM imports. Call `stdoutWrite` directly.

Default permission policy is permissive. A Program with no explicit grant subset may read and write the filesystem and listen and connect TCP on the targets that already expose those APIs. This is not a Deno deny-by-default. Opt-in `--allow-*` flags on `draconic run` install a grant subset when you want one. See [CLI](cli.html).

## stdoutWrite

`stdoutWrite` writes text or bytes to standard output. There is no automatic newline. Save this as `hello-host.drac`. It builds today:

```drac
stdoutWrite("hello from Draconic\n");
```

Parse it, typecheck it, or run it. Default `draconic run` target is js:

```
draconic parse hello-host.drac
draconic check hello-host.drac
draconic run hello-host.drac
```

## Filesystem

`readFileText` reads a whole file as text. `writeFileText` writes one. Those names are portable: both backends accept them. Save this as `note.drac`. It builds today:

```drac
writeFileText("note.txt", "hello from Draconic\n");
let text = readFileText("note.txt");
stdoutWrite(text);
```

Parse it, typecheck it, or run it:

```
draconic parse note.drac
draconic check note.drac
draconic run note.drac
```

## Sockets then HTTP

Networking is sockets-first. `tcpListen`, `tcpAccept`, `tcpConnect`, `tcpRead`, `tcpWrite`, and `closeTcp` land on both backends. HTTP/1.1 helpers `httpParseRequest` and `httpWriteResponse` sit on those sockets. The designed surface is not a Node-shaped `http` module as the only entry.

Listen and server paths started native-first. The JS backend now bridges those names, so a Program that uses `tcpListen` builds on js and on native. Host APIs that remain native-only still hard-error on js.

v1 HTTP is plaintext HTTP/1.1. TLS, HTTP/2, and WebSocket are later.

## tcpListen

`tcpListen` binds a TCP port. `closeTcp` releases it. Save this as `listen.drac`. It builds today:

```drac
let s = tcpListen(8080);
stdoutWrite("listening on 8080\n");
closeTcp(s);
```

Parse it, typecheck it, or emit a native binary:

```
draconic parse listen.drac
draconic check listen.drac
draconic build --target native listen.drac -o listen
./listen
```

## HTTP echo

A listen loop accepts one connection, parses one HTTP/1.1 request, writes the path as the body, and closes. Save this as `echo.drac`. It builds today:

```drac
let s = tcpListen(8080);
stdoutWrite("http-echo listening on 8080\n");
while (true) {
  let a = tcpAccept(s);
  let raw = tcpRead(a, 65536);
  let req = httpParseRequest(raw);
  let path = req.path;
  let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", path);
  tcpWrite(a, resp);
  closeTcp(a);
}
```

Parse it, typecheck it, or emit a native binary. This Program serves until you stop it, so build it rather than `draconic run` it in the foreground unless you mean to listen:

```
draconic parse echo.drac
draconic check echo.drac
draconic build --target native echo.drac -o echo
./echo
```

The in-repo copy is [HTTP echo](https://github.com/hembrow-innovations/draconic/tree/main/examples/http-echo). Copy [Flagship service](https://github.com/hembrow-innovations/draconic/tree/main/examples/flagship-service) when you want that listen path plus filesystem config and a git dependency.

Continue to [packages](packages.html). Lookup: [host I/O](reference-host-io.html).
