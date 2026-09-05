# Flagship service — typed HTTP + fs/config + git dep

One in-repo Program that combines **typed HTTP/1.1**, **filesystem config**, and a **git module path** (ADR-0008 / ADR-0009 / Roadmap **P04**).

Does not replace [`examples/http-echo`](../http-echo/) or [`examples/todo`](../todo/). Depends on [`examples/pkg-lib`](../pkg-lib/) via:

```
github.com/draconic-lang/pkg-lib
```

Reads `config.txt` with `readFileText`. Native listen is `server.drac`; the JS target runs the portable config/git-dep path in `portable.drac` (HTTP listen stays native-first).

## Layout

```
examples/flagship-service/
  draconic.toml    # module + dependency on pkg-lib
  config.txt       # fs config (greeting name)
  server.drac      # native HTTP + config + git dep
  portable.drac    # JS-runnable config + git dep (no listen)
  README.md
```

## Prerequisites

- Rust toolchain (`cargo`) — builds the `draconic` CLI
- LLVM toolchain — native target
- Node.js — portable JS path

## Documented build path

`pkg-lib` is not a published remote in this monorepo. Point `[urls]` (or `draconic get --url`) at a **local git** checkout of `examples/pkg-lib`, then tidy/fetch and build.

From the **repo root**:

```bash
# 1. Local git upstream from in-repo pkg-lib
UP=$(mktemp -d)/pkg-lib
mkdir -p "$UP"
cp examples/pkg-lib/draconic.toml examples/pkg-lib/index.drac examples/pkg-lib/README.md "$UP"/
git -C "$UP" init
git -C "$UP" checkout -B main
git -C "$UP" add .
git -C "$UP" -c user.email=dev@local -c user.name=dev commit -m v0.1.0
git -C "$UP" tag v0.1.0

# 2. Fetch into the service workspace (writes lock + cache)
cargo run -p draconic-cli -- get github.com/draconic-lang/pkg-lib@0.1.0 \
  --url "$UP" --dir examples/flagship-service
```

### Native HTTP (server.drac)

```bash
cargo run -p draconic-cli -- build --target native examples/flagship-service/server.drac \
  -o /tmp/flagship-service
(cd examples/flagship-service && /tmp/flagship-service)
```

In another terminal:

```bash
curl -s http://127.0.0.1:18084/hello
```

Expected response body:

```
hello, flagship 0.1.0 /hello
```

Server stdout (once):

```
flagship-service listening on 18084
```

Stop with Ctrl-C.

### JS portable path (portable.drac)

```bash
cargo run -p draconic-cli -- build --target js examples/flagship-service/portable.drac \
  -o /tmp/flagship-portable.js
(cd examples/flagship-service && node /tmp/flagship-portable.js)
```

Expected stdout:

```
hello, flagship 0.1.0
```

### Alternative: `mod tidy` with `[urls]`

Edit `examples/flagship-service/draconic.toml` and add:

```toml
[urls]
"github.com/draconic-lang/pkg-lib" = "/absolute/path/to/pkg-lib-upstream"
```

Then `draconic mod tidy --dir examples/flagship-service` and build as above.

## Tests

`tests/integration/tests/flagship_service.rs` (**P04**).
