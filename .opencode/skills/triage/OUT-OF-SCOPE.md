# Out-of-scope knowledge

Rejected feature requests are durable knowledge. They survive a clone. Write them through **docs**, not under `.heio/`.

They serve two purposes:

1. **Institutional memory.** Why a feature was rejected, so the reasoning isn't lost when the ticket is closed.
2. **Deduplication.** When a new ticket matches a prior rejection, surface the previous decision instead of re-litigating it.

Do not invent a folder under `.heio/` for this. **management** does not name one.

## Where the notes live

Load **docs**. Search `docs/` first for an existing rejection, out-of-scope, or "we will not build this" note. Update that note if you find one.

If `AGENTS.md` already names a docs layout for rejected work, that file wins.

If nothing exists, copy the matching **docs** template (usually a guide). Place and name it per **docs**. One note per concept, not per ticket. Multiple tickets requesting the same thing share one note.

## File format

Write in a relaxed, readable style. More like a short design document than a database entry. Use paragraphs, code samples, and examples so the reasoning is useful to someone encountering it for the first time.

```markdown
# Dark Mode

This project does not support dark mode or user-facing theming.

## Why this is out of scope

The rendering pipeline assumes a single color palette defined in
`ThemeConfig`. Supporting multiple themes would require:

- A theme context provider wrapping the entire component tree
- Per-component theme-aware style resolution
- A persistence layer for user theme preferences

This is a significant architectural change that doesn't align with the
project's focus on content authoring. Theming is a concern for downstream
consumers who embed or redistribute the output.

```ts
// The current ThemeConfig interface is not designed for runtime switching:
interface ThemeConfig {
  colors: ColorPalette; // single palette, resolved at build time
  fonts: FontStack;
}
```

## Prior requests

- `[[ticket-42-dark-mode]]`. "Add dark mode support"
- `[[ticket-87-night-theme]]`. "Night theme for accessibility"
- `[[ticket-134-dark-theme]]`. "Dark theme option"


### Naming the file

Use a short, descriptive kebab-case name for the concept: `dark-mode`, `plugin-system`, `graphql-api`. The name should be recognizable enough that someone browsing understands what was rejected without opening the file.

### Writing the reason

The reason should be substantive. Not "we don't want this" but why. Good reasons reference:

- Project scope or philosophy ("This project focuses on X. Theming is a downstream concern.")
- Technical constraints ("Supporting this would require Y, which conflicts with our Z architecture.")
- Strategic decisions ("We chose to use A instead of B because...")

The reason should be durable. Avoid referencing temporary circumstances ("we're too busy right now"). Those aren't rejections. They're deferrals.

## When to check

During triage (Step 1: Gather context), search `docs/` for rejection notes. When evaluating a new ticket:

- Check if the request matches an existing out-of-scope concept
- Matching is by concept similarity, not keyword. "night theme" matches a dark-mode note
- If there's a match, surface it to the maintainer: "This is similar to [[that-note]]. We rejected this before because [reason]. Do you still feel the same way?"

The maintainer may:

- **Confirm.** The new ticket gets added to the existing note's "Prior requests" list, then closed.
- **Reconsider.** The rejection note gets deleted or updated, and the ticket proceeds through normal triage.
- **Disagree.** The tickets are related but distinct. Proceed with normal triage.

## When to write

Only when an **enhancement** (not a bug) is rejected as `dropped`.

Do not write here when a ticket is `dropped` because it's already implemented. That's a built feature, not a rejected one. Recording it would poison the dedup check with false rejections. Point at where the feature lives in the closing `## Comments` note instead.

The flow:

1. Maintainer decides a feature request is out of scope
2. Search `docs/` for a matching rejection note
3. If yes: append the new ticket to the "Prior requests" list
4. If no: create a new note via **docs** with the concept name, decision, reason, and first prior request
5. Append a `## Comments` note on the ticket explaining the decision and `[[wikilink]]`-ing the rejection note
6. Set the ticket `status: dropped` and move it to `.heio/archive/planning/tickets/`

## Updating or removing rejection notes

If the maintainer changes their mind about a previously rejected concept:

- Delete or update the `docs/` note
- This skill does not need to reopen old tickets. They're historical records
- The new ticket that triggered the reconsideration proceeds through normal triage
