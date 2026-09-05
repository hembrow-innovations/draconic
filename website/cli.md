---
title: CLI
section: reference
status: shipped
---

# CLI

The toolchain command is `draconic`. A Program is the unit of source it accepts.

- `draconic parse <file>` — parse a Program and print the AST dump
- `draconic build --target js|native [--strip] [--lto] <file> [-o <out>]` — compile to JavaScript or a native binary. `--strip` and `--lto` are native-only size opts (LTO is a size-delta smoke versus the default native artifact).
- `draconic run [--target js|native] <file> [args...]` — build and execute a Program (default target: js)
- `draconic test <path>` — run conformance fixtures
- `draconic version` — print version
- `draconic help` — show usage

Shebang: `#!/usr/bin/env draconic` invokes `run` on the script path. In-repo example: `examples/shebang/hello.drac`. Make it executable and run it when `draconic` is on `PATH`:

```bash
chmod +x examples/shebang/hello.drac
./examples/shebang/hello.drac
```

`parse`, `build`, and `run` ship today. Package commands such as `get` and `mod tidy` are under [packages](reference-packages.html).

See also [types](types.html) and [Dual-world rules](dual-world-rules.html).
