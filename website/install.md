---
title: Install
section: learn
status: shipped
---

# Install

Get the toolchain, then parse, build, and run a Program before reading further.

Install:

```
curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh
```

That places `draconic` in `~/.draconic/bin`. Add that directory to `PATH` if needed, then confirm the binary:

```
export PATH="$HOME/.draconic/bin:$PATH"
draconic -V
```

The install script picks the host pair. Release CI builds a host-triple binary for each available OS/arch pair:

- linux/amd64
- linux/arm64
- darwin/amd64
- darwin/arm64
- windows/amd64
- windows/arm64

A Program is a unit of Draconic source the toolchain accepts. Save this as `hello.drac`. It builds today:

```drac
let console = globalThis.console;
console.log("hello from Draconic");
```

Parse it, build it to JavaScript, or run it. Default `draconic run` target is js:

```
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
draconic run hello.drac
```

Native binaries need an LLVM toolchain on the machine:

```
draconic build --target native hello.drac -o hello
./hello
```

The clone-build-run path stays in the repository README. Learn assumes you can already parse and build.

Next: pick a landing. Use [from JavaScript](from-javascript.html) if you already think in ECMAScript. Use [from systems](from-systems.html) if you already think in Rust, Go, or C. Those landings join at [Dual worlds](dual-worlds.html).

## Reproducibility

Same source plus a matching toolchain pin does not always mean byte-identical files. Use this policy to tell whether two artifacts should match.

- JS artifacts: byte-identical for the same Program source and pin. The JS backend does not embed timestamps or source paths.
- LLVM IR: identical for the same Program source, pin, and source path. Native DWARF records embed the source path (filename and directory), so a different checkout path produces different IR.
- Linked native binaries: Mach-O and ELF timestamps, UUIDs, and linker noise may differ across builds. Documented-equivalent for native is identical LLVM IR, not a byte-identical packaged binary.

Release artifacts copied by the install path keep the linked binary as the host toolchain wrote it. They are not normalized for timestamps or paths.
