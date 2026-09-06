---
name: domain-modeling
description: Build and sharpen a project's domain model in the committed docs/ vault. Use when the user wants to pin down domain terminology or a ubiquitous language, record an architectural decision, or when another skill needs to maintain the domain model in a repo that uses the docs skill. Prefer this over domain-modeling whenever docs/ follows that vault.
---

# Domain modeling in the docs vault

Actively build and sharpen the project's domain model as you design. Challenge terms. Invent edge-case scenarios. Write the glossary and decisions down the moment they crystallise.

Merely reading the glossary for vocabulary is not this skill. Any skill can do that in one line. This skill is for when you are changing the model.

The store is the `docs/` vault from the **docs** skill. Load **docs** before any write. Follow its write-before, layout, frontmatter, and wikilink rules. Do not invent a second layout. If `AGENTS.md` already names a different docs tree, that file wins.

Do not write `CONTEXT.md`, `CONTEXT-MAP.md`, or `docs/adr/`. Those belong to `/domain-modeling`. This skill does not keep a parallel copy.

## File structure

Most repos have a single context. The glossary is one overview note:

```
docs/
├── overview/
│   └── glossary.md
└── decisions/
    └── adr/
        ├── 0001-event-sourced-orders.md
        └── 0002-postgres-for-write-model.md
```

If `docs/overview/overview-context-map.md` exists, the repo has multiple contexts. The map lists each glossary. Domain stays in frontmatter. Do not invent `docs/<domain>/`.

```
docs/
├── overview/
│   ├── overview-context-map.md
│   ├── glossary-ordering.md
│   └── glossary-billing.md
└── decisions/
    └── adr/
```

Every ADR lives here. Set `domain:` to the context it belongs to.

Create files lazily, only when you have something to write. If no glossary exists, create one when the first term is resolved. If no `docs/decisions/adr/` exists, create it when the first ADR is needed.

### Infer the layout

- `docs/overview/overview-context-map.md` exists. Multi-context. Read the map. Edit the glossary the current topic belongs to. If that is unclear, ask.
- Only `docs/overview/glossary.md` exists. Single context.
- Neither exists. Create `docs/overview/glossary.md` on the first resolved term.

A leftover `CONTEXT.md` or `CONTEXT-MAP.md` is input, not the store. On the first write, move any still-true terms into the vault glossary. Then stop touching the leftover files.

## During the session

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in the glossary, call it out immediately. "Your glossary defines 'cancellation' as X, but you seem to mean Y. Which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'account'. Do you mean the Customer or the User? Those are different things."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it. "Your code cancels entire Orders, but you just said partial cancellation is possible. Which is right?"

### Update the glossary inline

When a term is resolved, update the glossary right there. Do not batch these up. Capture them as they happen. Use the format in [GLOSSARY-FORMAT.md](./GLOSSARY-FORMAT.md).

Copy the **docs** overview template for frontmatter and placement. The body is a glossary and nothing else. Do not treat it as a spec, a scratch pad, or a home for implementation decisions.

Link related notes with `[[wikilinks]]`. Never use relative `.md` paths.

### Offer ADRs sparingly

Only offer to create an ADR when all three are true:

1. **Hard to reverse.** The cost of changing your mind later is meaningful.
2. **Surprising without context.** A future reader will wonder why it was done this way.
3. **The result of a real trade-off.** There were genuine alternatives and you picked one for specific reasons.

If any of the three is missing, skip the ADR. Prefer a purpose, spec, standard, or system-design from **docs** when that kind fits better.

When you do write an ADR, copy `templates/adr.md` from the **docs** skill. Place it at `docs/decisions/adr/NNNN-<slug>.md`. Scan live files for the next number. Never renumber. Never delete. Set `domain:` to the context the decision belongs to. Delete unused template sections instead of leaving them empty. Wikilink the glossary and any related decision.

### What qualifies as an ADR

- Architectural shape.
- Integration patterns between contexts.
- Technology choices that carry lock-in. Not every library. The ones that would take a quarter to swap out.
- Boundary and scope decisions. The explicit nos are as valuable as the yeses.
- Deliberate deviations from the obvious path. These stop the next engineer from "fixing" something that was deliberate.
- Constraints not visible in the code.
- Rejected alternatives when the rejection is non-obvious.
