# FizzBuzz — portable CLI-style Program

Classic FizzBuzz (1..20) in **Draconic**, compiled to JavaScript and run with Node.

Exercises: `for`, `if`/`else`, `%`, string literals, `String(...)`, host I/O via `globalThis.console`.

## Prerequisites

- Rust toolchain (`cargo`) — builds the `draconic` CLI
- Node.js — runs the emitted JS

## Demo path

From the **repo root**:

```bash
cargo run -p draconic-cli -- build --target js examples/fizzbuzz/main.drac -o /tmp/fizzbuzz.js
node /tmp/fizzbuzz.js
```

Expected stdout:

```
1
2
Fizz
4
Buzz
Fizz
7
8
Fizz
Buzz
11
Fizz
13
14
FizzBuzz
16
17
Fizz
19
Buzz
```

## Layout

```
examples/fizzbuzz/
  main.drac   # source of truth
  README.md
```

## Notes

- **JS target:** supported and demonstrated above.
- **Native target:** not required for this example. Native has its own print hooks for top-level scalars; `globalThis.console` is a JS/host pattern (same as `examples/todo`).
- Free identifier `console` is unresolved — bind via `globalThis.console` (see `main.drac`).
