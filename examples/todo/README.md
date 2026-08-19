# Todo — Draconic end-to-end example

A browser todo list written in **Draconic**, compiled to JavaScript, and served by a **pure Draconic native** static HTTP server (no C host).

```
src/todo.drac   ──draconic build --target js──►  public/todo.js
public/index.html + style.css                    (static shell)
server.drac     ──draconic build --target native──►  native binary hosts HTTP
```

## Prerequisites

- Rust toolchain (`cargo`) — builds the `draconic` CLI
- LLVM toolchain — native target (same as other native builds)
- Browser

## Quick start

From the **repo root**:

```bash
./examples/todo/build.sh
(cd examples/todo && ./server-bin)
```

Open [http://127.0.0.1:18083/](http://127.0.0.1:18083/).

Or from this directory:

```bash
./build.sh
./server-bin
```

## What is Draconic vs host

| Piece | Language | Role |
|-------|----------|------|
| `src/todo.drac` | Draconic | App logic + DOM via `globalThis` |
| `public/todo.js` | Emitted JS | Browser script (generated; do not edit) |
| `public/index.html`, `style.css` | HTML/CSS | Page shell |
| `server.drac` | Draconic | Native HTTP/1.1 static file host (TCP + `httpServeStatic`) |

The server is **native-only**. The **client** is the JS product of the language: types, arrays, `JSON`, and browser APIs via `globalThis`.

## Features

- Add / toggle / delete todos
- Filters: All · Active · Completed
- Remaining count + clear completed
- Persist to `localStorage` (`draconic-todo`)

## Rebuild pieces

```bash
# from repo root — client only
cargo run -q -p draconic-cli -- build --target js \
  examples/todo/src/todo.drac -o examples/todo/public/todo.js

# server only
cargo run -q -p draconic-cli -- build --target native \
  examples/todo/server.drac -o examples/todo/server-bin
```

## Layout

```
examples/todo/
  src/todo.drac       # client source of truth
  public/
    index.html
    style.css
    todo.js           # generated
  server.drac         # native static host source of truth
  build.sh
  README.md
```

## Notes

- **Native target** for `server.drac` (TCP listen is native-first).
- Port is fixed at `18083` in `server.drac` (avoids clash with http-echo on 8080).
- Docroot is `./public` relative to the process cwd — run from `examples/todo`.
- One request per connection (`Connection: close`).
- Path traversal (`..`) and non-files under docroot → 404.
- Integration: `tests/integration/tests/todo_server.rs` (**H17.03**).
