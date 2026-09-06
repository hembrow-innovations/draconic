# Stitch prompts

Enhance every brief before `generate` or `edit`. Include vibe, layout, components, device, and constraints. Device is **DESKTOP** unless DESIGN.md or the user names another.

Stay on this dest's product. Read `.stitch/DESIGN.md` if it exists. If the dest has product UI docs (AGENTS.md, a location note, a brief), use those as constraints.

## Structure (new screens)

```
[product] [screen]: [one line purpose + vibe].

**PLATFORM:** [desktop app | web | mobile]
**DEVICE:** DESKTOP

**DESIGN SYSTEM (REQUIRED):**
- Theme: [light/dark], [density / tone]
- Background: [Descriptive name] (#hex) for [role]
- Primary Accent: [Descriptive name] (#hex) for [role]
- Text Primary: [Descriptive name] (#hex)
- [more tokens]

**PAGE STRUCTURE:**
1. **Chrome:** [wordmark, title, primary nav]
2. **Primary pane:** [the one screen this prompt is for]
3. **Status / secondary:** [what must stay visible]
4. **Detail:** [table, list, canvas, form]
```

If `.stitch/DESIGN.md` exists, paste its tokens into **DESIGN SYSTEM** and keep later screens on that language. Skip hex/fonts in a generate prompt only when a Stitch design system is already applied on the project.

## Vague → specific

- **"nice header"** → named chrome: wordmark, title, the one primary action
- **"dashboard"** → one screen with a purpose; pick the pane
- **"list of items"** → table or list — pick one and name the row
- **"button"** → one labeled action and where it lives
- **"dark mode"** → dark theme plus contrast needs (logs, tables, body)

## Edits

One change at a time. Name location, then the change.

- **Location**: "Change the primary button in the app chrome…"
- **Visuals**: "…to a quieter accent (#hex) and a 1px divider."
- **Structure**: "Add a status badge next to the title."

Do not regenerate because a button is wrong.

## Variants

- **REFINE**: subtle
- **EXPLORE**: balanced (default)
- **REIMAGINE**: radical

Aspects: `LAYOUT`, `COLOR_SCHEME`, `IMAGES`, `TEXT_FONT`, `TEXT_CONTENT`. Leave empty for all.

## Device

- **DESKTOP**: default. Wide panes, tables, app chrome.
- **TABLET**: only if DESIGN.md or the user asks
- **MOBILE**: only if DESIGN.md or the user asks
- **AGNOSTIC**: only if DESIGN.md or the user asks
