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

## processArgs

`processArgs()` returns leftover user args after `draconic run file.drac` as a string array. `envGet(key)` reads a string or undefined. `envSet(key, value)` writes one. `envDelete(key)` removes one. `exit(code)` terminates. `stdinReadLine()` reads one line. Save this as `args.drac`. It builds today:

```drac
let args = processArgs();
stdoutWrite("hello from processArgs\n");
```

```
draconic parse args.drac
draconic check args.drac
draconic run args.drac
```

## Names

- Process and stdio: `processArgs()`, `envGet(key)`, `envSet(key, value)`, `envDelete(key)`, `exit(code)`, `stdinReadLine()`, `stdoutWrite`, `stderrWrite`
- Filesystem: `readFileText(path)`, `writeFileText(path, text)`
- TCP sockets: `tcpListen(port)`, `tcpAccept(listen)`, `tcpConnect(host, port)`, `tcpRead(connection, maxLen)`, `tcpWrite(connection, bytes)`, `closeTcp(handle)`
- HTTP/1.1 helpers on those sockets: `httpParseRequest(raw)`, `httpWriteResponse(status, reason, headers, body)` — not a Node-shaped http module as the only entry

`httpParseRequest` returns a request with `method`, `path`, `version`, and `body`. `tcpRead` reads up to `maxLen` bytes.

Listen and server paths started native-first. Those names also build on the JS backend through an explicit bridge. Native-only host APIs still hard-error on js.

## HTTP echo

Accept, parse one request, write the path as the body, close. Save this as `echo.drac`. It builds today:

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

```
draconic parse echo.drac
draconic check echo.drac
draconic build --target native echo.drac -o echo
./echo
```

Default permission policy is permissive; `--allow-*` on `draconic run` installs an opt-in grant subset.

v1 HTTP is plaintext HTTP/1.1. TLS, HTTP/2, and WebSocket are later.

Command flags: [CLI](cli.html).
