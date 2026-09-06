---
title: Read purpose, then contracts, then tests
impact: CRITICAL
tags: [read]
---

# Read purpose, then contracts, then tests

Before coding or changing behaviour, walk the ladder for that area.

1. Read `purpose.md` in full. Out of scope is a hard fence. A non-empty Open product questions section means stop.
2. Read every `contract.md` in that folder (and nested feature folders you will touch).
3. Read the matching `test.md` notes. They tell you which code tests already lock the promises.
4. Name the **promise ids** you will keep or change.
5. Then open code.

Must-read is purpose plus the contracts for the unit. test.md is next. ADRs and guides are related, not a substitute.

Empty ladder (no purpose, no contracts, or they do not answer) → stop. Open a ticket through the project's tracker, or assert a promise (`contract-anatomy`). Never invent product rules. `principle-intent-ladder-stop` owns that stop if it is installed.

A context packer (**vault-pack**, `vault:pack`, or whatever `package.json` names) may assemble this set. Use it when it exists. Still Read every Must-read path. A CLI run is not a read.

Done when you can point at promise ids, or you have stopped.
