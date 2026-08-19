# pkg-consumer — depends on pkg-lib

In-repo demo consumer for Go-style git packages (ADR-0009 / Roadmap **K10.02**).

Depends on [`examples/pkg-lib`](../pkg-lib/) via module path:

```
github.com/draconic-lang/pkg-lib
```

## Layout

```
examples/pkg-consumer/
  draconic.toml   # module + dependency on pkg-lib
  main.drac       # import { greet, VERSION } from module path
  README.md
```

## Documented build path

`pkg-lib` is not a published remote in this monorepo. Point `[urls]` (or
`draconic get --url`) at a **local git** checkout of `examples/pkg-lib`, then
tidy/fetch and build.

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

# 2. Fetch into consumer workspace (writes lock + cache; keeps manifest dep)
cargo run -p draconic-cli -- get github.com/draconic-lang/pkg-lib@0.1.0 \
  --url "$UP" --dir examples/pkg-consumer

# 3. Build + run (JS target)
cargo run -p draconic-cli -- build --target js examples/pkg-consumer/main.drac \
  -o /tmp/pkg-consumer.js
node /tmp/pkg-consumer.js
```

Expected stdout:

```
0.1.0
hello, pkg-consumer
```

### Alternative: `mod tidy` with `[urls]`

Edit `examples/pkg-consumer/draconic.toml` and add:

```toml
[urls]
"github.com/draconic-lang/pkg-lib" = "/absolute/path/to/pkg-lib-upstream"
```

Then:

```bash
cargo run -p draconic-cli -- mod tidy --dir examples/pkg-consumer
cargo run -p draconic-cli -- build --target js examples/pkg-consumer/main.drac \
  -o /tmp/pkg-consumer.js
node /tmp/pkg-consumer.js
```

## Tests

`tests/packages` — `k10_02_pkg_consumer` (layout + documented build path e2e).
