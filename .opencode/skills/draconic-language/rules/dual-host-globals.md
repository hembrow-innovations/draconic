---
title: Host APIs are free globals
impact: CRITICAL
impactDescription: Importing host I/O invents a module that does not exist
tags: [dual, host, globals]
---

## Host APIs are free globals

Host APIs are free identifiers on the global object, registered in the Checker (`host_api.rs`). They are not ESM imports.

**Incorrect:**

```
import { tcpListen } from "draconic:host";
```

**Correct:**

```
tcpListen(8080);
```

**Notes.** Unavailable on a target → E0400 `HOST_API_UNSUPPORTED`. JS target gets Node bridges prepended when IR references those names. See `host-sockets-first` and `js-polyfill-runtime`.
