# Host I/O: sockets-first, then thin HTTP

Host networking and I/O land under Roadmap **H**. **Sockets first** (TCP listen/accept/connect/read/write on native), then thin HTTP/1.1 request/response helpers on those sockets—not a Node-shaped `http` module as the only entry, and not HTTP-only with sockets forever private.

**Targets:** native first for listen/server paths; JS hard-errors unsupported host APIs until an explicit bridge row. v1 HTTP is plaintext HTTP/1.1; TLS, HTTP/2, and WebSocket are later **H** clusters. Success Program: pure-Draconic `examples/http-echo`; later cutover of `examples/todo` off the C host.

Rejected: Deno-only `serve` without a socket layer; npm-registry-first networking; claiming e2e HTTP while a non-Draconic host process owns listen/accept.
