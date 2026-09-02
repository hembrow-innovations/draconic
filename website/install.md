---
title: Install
section: learn
status: shipped
---

# Install

Get the toolchain, then parse and build a Program before reading further.

Install:

```
curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh
```

That places `draconic` in `~/.draconic/bin`. Add that directory to `PATH` if needed.

The install script picks the host pair. Release CI builds a host-triple binary for each available OS/arch pair:

- linux/amd64
- linux/arm64
- darwin/amd64
- darwin/arm64
- windows/amd64
- windows/arm64

A Program is a unit of Draconic source the toolchain accepts. Save this as `hello.drac`. It builds today:

```drac
let sample = 1 + 2;
```

Parse it, then build it to JavaScript:

```
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
```

Native binaries need an LLVM toolchain:

```
draconic build --target native hello.drac -o hello
```

The clone-build-run path stays in the repository README. Learn assumes you can already parse and build.
