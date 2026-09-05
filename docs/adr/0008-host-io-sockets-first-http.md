# Host I/O: sockets-first, then thin HTTP

Host networking and I/O land under Roadmap **H**. **Sockets first** (TCP listen/accept/connect/read/write on native), then thin HTTP/1.1 request/response helpers on those sockets—not a Node-shaped `http` module as the only entry, and not HTTP-only with sockets forever private.

**Targets:** native first for listen/server paths; JS hard-errors unsupported host APIs until an explicit bridge row. v1 HTTP is plaintext HTTP/1.1; TLS, HTTP/2, and WebSocket are later **H** clusters. Success Program: pure-Draconic `examples/http-echo`; later cutover of `examples/todo` off the C host.

Rejected: Deno-only `serve` without a socket layer; npm-registry-first networking; claiming e2e HTTP while a non-Draconic host process owns listen/accept.

**Default permission policy (R02.04):** permissive. A Program with no explicit grant subset may read/write the filesystem and listen/connect TCP on the targets that already expose those host APIs (H04 / H06; JS TCP via the H17.04 bridge). There is no locked-down deny-by-default until opt-in grants land (R02.01–R02.03). Rejected: treating the designed default as Deno-style locked-down while host surfaces already succeed without grants.

**Permission grants (R02.01):** an explicit grant subset (`fs-read`, `fs-write`, `net-listen`, `net-connect`) is forwarded as `DRACONIC_PERMISSIONS` and those host ops succeed. CLI flags that install the subset are R02.03; deny diagnostics when a grant is missing are R02.02.
