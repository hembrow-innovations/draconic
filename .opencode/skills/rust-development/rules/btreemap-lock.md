---
title: BTreeMap for manifests and locks
impact: MEDIUM
impactDescription: HashMap lockfiles rewrite on every tidy
tags: [pkg, btreemap, lock]
---

## BTreeMap for manifests and locks

Manifests and lockfiles use `BTreeMap` so serialize order is stable. Compiler tables use `HashMap`.

**Incorrect:**

```rust
pub struct Lockfile {
    pub packages: HashMap<String, LockEntry>,
}
```

**Correct:**

```rust
use std::collections::BTreeMap;

pub struct Lockfile {
    pub packages: BTreeMap<String, LockEntry>,
}
```

**Notes.** K02.03: rewrite of an unchanged lock is byte-identical; packages sorted by path. Redact git auth; do not put secrets in lockfiles. See `err-pkg-enum`.
