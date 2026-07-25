# Shared IR and dual backends

After the Frontend, all Programs lower to one shared IR. The JS backend and LLVM backend both consume that IR. Forking from a typed AST per backend was rejected because semantics would drift; targeting a foreign IR (WASM-only, etc.) was rejected because GC, Embed, and JS-faithful behavior need full control.
