# pkg-lib — minimal exportable module

In-repo demo library for Go-style git packages (ADR-0009 / Roadmap **K10.01**).

Exports:

| Name | Kind | Meaning |
|------|------|---------|
| `VERSION` | `const` string | Package version label |
| `greet(name)` | function | Returns `"hello, " + name` |

## Layout

```
examples/pkg-lib/
  draconic.toml   # module = "github.com/draconic-lang/pkg-lib"
  index.drac      # package root entry (named exports)
  README.md
```

## Module path

```
github.com/draconic-lang/pkg-lib
```

Consumers import:

```js
import { greet, VERSION } from "github.com/draconic-lang/pkg-lib";
```

## Local use (until published as a real git remote)

Point a consumer `[urls]` map at a git clone of this tree (or a path URL after
`git init` + tag), then `draconic mod tidy` / build. Full consumer demo:
[`examples/pkg-consumer`](../pkg-consumer/) (**K10.02**).

## Tests

`tests/packages` — `k10_01_pkg_lib` (layout + import via module path).
