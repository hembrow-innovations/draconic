---
title: Host APIs are free globals in the registry
impact: HIGH
impactDescription: Import-based host APIs fight H00
tags: [host, check, registry]
---

## Host APIs are free globals in the registry

Host APIs are free identifiers on the global object, registered in `draconic-check` `host_api.rs` with `HostAvailability::NATIVE_ONLY` or `BOTH`. The checker owns the name registry.

**Incorrect:**

```rust
import { readFile } from "draconic:fs";
```

as the product surface, or a one-off name check in the JS emitter.

**Correct:**

```rust
HostApiEntry {
    name: "readFile",
    availability: HostAvailability::NATIVE_ONLY,
    /* … */
}
```

**Notes.** Native-only entries hard-error on js (ADR-0008). H17.04 Node bridges exist only where that row landed. Add the registry row before emit. See `host-permissive` and `dual-js-policy`.
