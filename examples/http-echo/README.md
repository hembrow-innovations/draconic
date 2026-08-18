# HTTP echo — pure Draconic native server

Plaintext HTTP/1.1 echo server in **Draconic**, compiled to a native binary. No C host process owns listen/accept — the Program uses host TCP + thin HTTP helpers (ADR-0008 / Roadmap **H17.01**).

Each accepted connection: parse one request → respond `200` with the request path as the body → close.

## Prerequisites

- Rust toolchain (`cargo`) — builds the `draconic` CLI
- LLVM toolchain — native target (same as other native builds)

## Demo path

From the **repo root**:

```bash
cargo run -p draconic-cli -- build --target native examples/http-echo/main.drac -o /tmp/http-echo
/tmp/http-echo
```

In another terminal:

```bash
curl -s http://127.0.0.1:8080/hello
```

Expected response body:

```
/hello
```

Server stdout (once):

```
http-echo listening on 8080
```

Stop with Ctrl-C.

## Layout

```
examples/http-echo/
  main.drac   # source of truth
  README.md
```

## Notes

- **Native target:** required (TCP listen is native-first; js hard-errors host listen APIs until a bridge row).
- Port is fixed at `8080` in `main.drac`.
- One request per connection (no keep-alive loop in this example).
- Full client/assert/shutdown integration is **H17.02**.
