---
title: Changing behaviour edits a promise
impact: HIGH
tags: [change]
---

# Changing behaviour edits a promise

Read the ladder first (`ladder-read`). Then:

- In scope and a named promise already covers it → keep that id. Implement against it.
- In scope and no promise covers it → write the promise first (`contract-anatomy`). Then the red test, then the code, then `test.md`.
- Out of scope → stop. Do not "fix it so it just works".
- A locked promise would become false → either the test fails (good) or you edit the promise line on purpose and say why.

New behaviour is contract-first: promise → red test (**tdd**) → green impl → harden with support tests → gate. Support tests need not map to a promise.

Never rename or delete a test that a promise points at without updating the contract.

The test: can you point at a promise id? If not, you are freestyling.

Ids, wording, grain, inheritance, checker: `contract-anatomy`, `contract-wording`, `contract-grain`, `contract-inherit`, `contract-gate`.

`principle-intent-ladder-stop` owns the stop when it is installed. Execution of a named promise proceeds. Inventing the promise does not.
