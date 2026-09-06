# Writing agent briefs

An agent brief is a structured `## Agent Brief` section appended to the task note when it is published `status: ready` and `mode: afk`. It is the authoritative specification that an AFK agent will work from. The ticket body is context. The agent brief is the contract.

Tickets live under `.heio/` via **management**. Do not write briefs into `docs/`.

## Principles

### Durability over precision

The task may sit `ready` for days or weeks. The codebase will change in the meantime. Write the brief so it stays useful even as files are renamed, moved, or refactored.

- **Do** describe interfaces, types, and behavioral contracts
- **Do** name specific types, function signatures, or config shapes that the agent should look for or modify
- **Don't** reference file paths. They go stale
- **Don't** reference line numbers
- **Don't** assume the current implementation structure will remain the same

### Behavioral, not procedural

Describe **what** the system should do, not **how** to implement it. The agent will explore the codebase fresh and make its own implementation decisions.

- **Good:** "The `SkillConfig` type should accept an optional `schedule` field of type `CronExpression`"
- **Bad:** "Open src/types/skill.ts and add a schedule field on line 42"
- **Good:** "When a user runs `/triage` with no arguments, they should see a summary of tickets needing attention"
- **Bad:** "Add a switch statement in the main handler function"

### Complete acceptance criteria

The agent needs to know when it's done. Every agent brief must have concrete, testable acceptance criteria. Each criterion should be independently verifiable.

- **Good:** "Running `/triage` with no arguments lists every ticket note tagged `needs-triage`"
- **Bad:** "Triage should work correctly"

### Explicit scope boundaries

State what is out of scope. This prevents the agent from adding extras or making assumptions about adjacent features.

### Intent system (behaviour changes)

Living product intent lives under `docs/`. Load **docs** and discover purpose, contracts, and specs. Do not assume a life-engine path.

When the work changes **what** the product must do, the brief is **incomplete** (do not publish `status: ready`) unless it includes all of:

1. **Contract promise ids** (for example `tasks.crud:complete`) when the project has them
2. **Purpose** wikilink for non-goals and scope fences
3. **Contract-first** acceptance: edit or assert promise, then test, then code
4. If the docs have no answer: "open a ticket / assert a promise". Never invent rules

Data-placement-only work: cite the project's data-flow or architecture note under `docs/` instead of freestyling layers. Pure refactors with no behaviour change may omit promise ids but must say so explicitly (**No product behaviour change**).

## Template

```markdown
## Agent Brief

**Category:** bug / enhancement
**Summary:** one-line description of what needs to happen

**Intent (required when product behaviour changes):**
- Promise ids: `area.section:name`, …
- Purpose: wikilink discovered under `docs/`
- Contract-first: edit/assert promise → test → code
- Or: **No product behaviour change** (refactor/docs only)

**Current behavior:**
Describe what happens now. For bugs, this is the broken behavior.
For enhancements, this is the status quo the feature builds on.

**Desired behavior:**
Describe what should happen after the agent's work is complete.
Be specific about edge cases and error conditions.

**Key interfaces:**
- `TypeName`. What needs to change and why.
- `functionName()` return type. What it currently returns vs what it should return.
- Config shape. Any new configuration options needed.

**Acceptance criteria:**
- [ ] Specific, testable criterion 1
- [ ] Specific, testable criterion 2
- [ ] Promise ids listed above still hold (or were deliberately edited)

**Out of scope:**
- Thing that should NOT be changed or addressed in this ticket
- Adjacent feature that might seem related but is separate
- Respect purpose **Out of scope** fences
```

## Examples

### Good agent brief (bug)

```markdown
## Agent Brief

**Category:** bug
**Summary:** Skill description truncation drops mid-word, producing broken output

**Current behavior:**
When a skill description exceeds 1024 characters, it is truncated at exactly
1024 characters regardless of word boundaries. This produces descriptions
that end mid-word (e.g. "Use when the user wants to confi").

**Desired behavior:**
Truncation should break at the last word boundary before 1024 characters
and append "..." to indicate truncation.

**Key interfaces:**
- The `SkillMetadata` type's `description` field. No type change needed,
  but the validation/processing logic that populates it needs to respect
  word boundaries
- Any function that reads SKILL.md frontmatter and extracts the description

**Acceptance criteria:**
- [ ] Descriptions under 1024 chars are unchanged
- [ ] Descriptions over 1024 chars are truncated at the last word boundary
      before 1024 chars
- [ ] Truncated descriptions end with "..."
- [ ] The total length including "..." does not exceed 1024 chars

**Out of scope:**
- Changing the 1024 char limit itself
- Multi-line description support
```

### Good agent brief (enhancement)

```markdown
## Agent Brief

**Category:** enhancement
**Summary:** Add durable rejection notes for tracking rejected feature requests

**Current behavior:**
When a feature request is rejected, the ticket is set `dropped`
and a comment is appended. There is no persistent record of the decision
or reasoning. Future similar requests require the maintainer to recall
or search for the prior discussion.

**Desired behavior:**
Rejected feature requests should be documented as durable notes under
`docs/` that capture the decision, reasoning, and links to all tickets that
requested the feature. When triaging new tickets, these notes should be
checked for matches.

**Key interfaces:**
- Note format via the **docs** skill. Each file should have a
  `# Concept Name` heading, a decision, a reason, and a prior-requests
  list with ticket wikilinks
- The triage workflow should read existing rejection notes early
  and match incoming tickets against them by concept similarity

**Acceptance criteria:**
- [ ] Dropping a feature creates or updates a rejection note under `docs/`
- [ ] The note includes the decision, reasoning, and link to the closed ticket
- [ ] If a matching rejection note already exists, the new ticket is
      appended to its prior-requests list rather than creating a duplicate
- [ ] During triage, existing rejection notes are checked and surfaced
      when a new ticket matches a prior rejection

**Out of scope:**
- Automated matching (human confirms the match)
- Reopening previously rejected features
- Bug reports (only enhancement rejections go to rejection notes)
```

### Bad agent brief

```markdown
## Agent Brief

**Summary:** Fix the triage bug

**What to do:**
The triage thing is broken. Look at the main file and fix it.
The function around line 150 has the ticket.

**Files to change:**
- src/triage/handler.ts (line 150)
- src/types.ts (line 42)
```

This is bad because:

- No category
- Vague description ("the triage thing is broken")
- References file paths and line numbers that will go stale
- No acceptance criteria
- No scope boundaries
- No description of current vs desired behavior
