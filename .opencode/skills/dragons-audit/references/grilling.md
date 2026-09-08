# Grill

Walk the picked finding or deepening candidate with the user. One question at a time. Do not load other skills.

## Cover until decided

- **Constraints** any change must keep
- **Dependencies** and their category from `deepening.md`
- **Shape** of the deepened module: small interface, what the implementation hides
- **Seam**: what sits behind it; which adapters are real (need two)
- **Tests**: which survive, which die, which rewrite at the new interface

Completion: each bullet has an answer, or the user stops.

## Domain writes (inline)

- Naming a deepened module after a concept not in `CONTEXT.md` or `docs/overview/glossary.md`? Add the term to whichever glossary exists. Create `CONTEXT.md` only if neither exists and a term actually resolved.
- Sharpening a fuzzy term? Update that glossary right there.
- User rejects with a load-bearing reason? Offer an ADR in the existing adr tree (`docs/adr/` or `docs/decisions/adr/`). Skip ephemeral reasons ("not now") and self-evident ones.

## Next

Want alternative interfaces? Read `design-it-twice.md` in this folder.
