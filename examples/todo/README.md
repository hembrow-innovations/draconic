# Todo — Draconic end-to-end example

A browser todo list written in **Draconic**, compiled to JavaScript, and served by a **native** static HTTP server.

```
src/todo.drac  ──draconic build --target js──►  public/todo.js
public/index.html + style.css                   (static shell)
server/main.c  ──cc──►  native binary hosts HTTP on :8080
```

## Prerequisites

- Rust toolchain (`cargo`) — builds the `draconic` CLI
- C compiler (`cc`) — builds the native host
- Browser

## Quick start

From the **repo root**:

```bash
./examples/todo/build.sh
./examples/todo/server/server 8080 ./examples/todo/public
```

Open [http://127.0.0.1:8080/](http://127.0.0.1:8080/).

Or from this directory:

```bash
./build.sh
./server/server 8080 ./public
```

## What is Draconic vs host

| Piece | Language | Role |
|-------|----------|------|
| `src/todo.drac` | Draconic | App logic + DOM via `globalThis` |
| `public/todo.js` | Emitted JS | Browser script (generated; do not edit) |
| `public/index.html`, `style.css` | HTML/CSS | Page shell |
| `server/main.c` | C | Native HTTP/1.1 static file host |

The Draconic **native** backend does not yet expose sockets/HTTP in the Runtime, so the host is a small C server. The **product** of the language here is the client: types, classes-of-logic, arrays, `JSON`, and browser APIs reached through `globalThis`.

## Features

- Add / toggle / delete todos
- Filters: All · Active · Completed
- Remaining count + clear completed
- Persist to `localStorage` (`draconic-todo`)

## Rebuild client only

```bash
# from repo root
cargo run -q -p draconic-cli -- build --target js \
  examples/todo/src/todo.drac -o examples/todo/public/todo.js
```

## Layout

```
examples/todo/
  src/todo.drac       # source of truth for the app
  public/
    index.html
    style.css
    todo.js           # generated
  server/
    main.c            # native static host
    Makefile
  build.sh
  README.md
```
