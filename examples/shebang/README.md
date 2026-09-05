# Shebang — `#!/usr/bin/env draconic`

A Program a stranger can `chmod +x` and execute. The first line is the documented shebang; the toolchain treats that invoke as `draconic run` (U14, default target js).

## Prerequisites

- The `draconic` CLI on `PATH` (install or `cargo build -p draconic-cli --release` and put the binary dir on `PATH`)

## Demo path

From the **repo root**:

```bash
chmod +x examples/shebang/hello.drac
./examples/shebang/hello.drac
```

Same Program via the U14 command:

```bash
draconic run examples/shebang/hello.drac
```

Expected stdout:

```
hello-shebang
```

## Layout

- **hello.drac**: source of truth; starts with `#!/usr/bin/env draconic`
- **README.md**: this file
