---
title: Three test surfaces
impact: HIGH
tags: [test]
---

# Three test surfaces

Do not mix these.

- **Code tests** — `it` / `test` / `describe` titles in the repo. The lock surface. Load **tdd** when writing them.
- **`test:` pointers** on `contract.md` — a substring of a real title. Present ⇒ the promise is **locked**. Absent ⇒ **asserted**. See `contract-anatomy`.
- **`test.md`** — the human map. Paths, titles, how, why, honest gaps. It does not lock anything.

Copy **docs** `templates/test.md`. Place it beside the `contract.md` it covers.

## Coverage

What this folder's tests are for. Name the promise ids they lock.

## Tests

- **path/to/file.test.ts** — `it` title
  - **How:** what the case sets up and asserts
  - **Why:** which promise or risk this exists for

## Gaps

Honest holes. No test yet for promise `feature.section:id`.

Same promise / more cases → more `test:` lines on the contract and more rows here, not more promises. Support tests need not map to a promise. Say so in Why.

Never rename or delete a test that a promise points at without updating the contract. Update `test.md` in the same change.
