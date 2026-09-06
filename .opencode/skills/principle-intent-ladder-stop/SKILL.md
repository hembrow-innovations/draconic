---
name: principle-intent-ladder-stop
description: "Apply when changing product behaviour or presentation, when purpose or contracts are missing, or when Open product questions is non-empty. If the intent ladder does not answer, stop. Do not invent product rules."
disable-model-invocation: true
---

# Intent Ladder Stop

If purpose and contracts do not answer the question, stop. Open a ticket or assert a promise. Do not invent product rules.

**Why:** Prose that is not locked will drift. A plausible "just make it work" change is how agents invent a second product.

**Pattern:**

- Read purpose first. Out of scope is a hard fence.
- Name the contract promise ids the change must keep or edit.
- Empty ladder or a non-empty Open product questions section means stop.
- Changing behaviour means editing a promise line first, then the test, then the code.
- Do not "fix it so it just works" under an existing locked promise.

**This overrides never-block-on-the-human for product direction.** Execution of a named promise proceeds. Inventing the promise does not.

**Delegate:** load `spec` for the folder, read order, and promise format. Load `vault-pack` for the pack. Do not restate them here.

**The test:** can you point at a promise id? If not, you are freestyling.
