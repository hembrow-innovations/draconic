---
title: Promises, locked and asserted
impact: HIGH
tags: [contract]
---

# Promises, locked and asserted

You can't lock prose. Prose rots. Lock **behaviour** in tests. Keep one thin `contract.md` per unit stating what/why (not how), wired to those tests so it can't drift silently. Two jobs: **readable** (a human understands the promises) and **locked** (deviating takes a deliberate, visible edit, not a silent code change).

Copy **docs** `templates/contract.md`. `tags: [contract]`. Wikilink purpose and test.md after copy.

```md
- `feature.section:promise-id`: One plain-language promise a human would state.
  test: {substring of a real it/test/describe title}   ← LOCKED
  test: {another case. one promise, many tests}
- `feature.section:other`: A promise with no test yet.  ← ASSERTED
```

- **id.** Namespaced `feature.section:name`, globally unique. Stable target for inheritance and overrides (`except records-table:delete`).
- **`test:` pointer present ⇒ locked.** The checker verifies each pointer matches a real `it` / `test` / `describe` title. No pointer ⇒ **asserted** (skipped: promised, not yet proven).
- **`contract_default: locked`** in frontmatter cranks the whole file rigid. Every promise must then be locked or the gate fails. Loose by default. Use on small, high-stakes files only, not large inventory contracts.

Wording: `contract-wording`. Grain: `contract-grain`. Gate: `contract-gate`.
