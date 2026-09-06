---
name: stitch
description: Stitch generate, edit, and pull UI screens via Google Stitch. Use when the user wants Stitch mockups, DESIGN.md, or a Stitch brief — never stitch.withgoogle.com.
---

# Stitch

Drive Google Stitch with the SDK CLI. Pi has no MCP.

## Setup (once)

Create a key at [stitch.withgoogle.com/settings](https://stitch.withgoogle.com/settings). Store it as `STITCH_API_KEY` in the environment or a gitignored `.env`. Never commit it. Never put it in a prompt, file under version control, or a browser app.

Install the SDK into this skill:

```
npm install --prefix .opencode/skills/stitch
```

Confirm:

```
node .opencode/skills/stitch/scripts/stitch.mjs doctor
```

Done when doctor prints `"ok": true`. If `"apiKey": false`, stop and ask the user for the key.

## CLI

Run from the dest root. JSON on stdout. Generation can take minutes — use a bash timeout of at least 360s.

```
node .opencode/skills/stitch/scripts/stitch.mjs <command> [flags]
```

- **doctor**: SDK + key present
- **projects**: list
- **create --title <name>**: new project
- **ensure --title <name>**: reuse matching title or create; writes `.stitch/metadata.json`
- **screens --project <id>**: list screens
- **generate --project <id> --prompt <text>|--prompt-file <path> [--device MOBILE|DESKTOP|TABLET|AGNOSTIC] [--model GEMINI_3_FLASH|GEMINI_3_PRO|GEMINI_3_1_PRO] [--pull] [--slug <slug>]**
- **edit --project <id> --screen <id> --prompt <text>|--prompt-file <path> [--pull]**
- **variants --project <id> --screen <id> --prompt <text> [--count 1-5] [--range REFINE|EXPLORE|REIMAGINE] [--aspects LAYOUT,COLOR_SCHEME,IMAGES,TEXT_FONT,TEXT_CONTENT] [--pull]**
- **get --project <id> --screen <id>**: HTML + screenshot download URLs
- **pull --project <id> --screen <id> [--slug <slug>] [--out .stitch/designs]**: fetch files to disk
- **pull --project <id> --all [--out .stitch/designs]**: every screen
- **upload --project <id> --file <path> [--title <name>] [--pull]**: image/HTML mockup → screen
- **upload-md --project <id> --file .stitch/DESIGN.md**
- **tools**: MCP tool names (escape hatch)
- **call --tool <name> --args '<json>'**: raw tool call

Prefer `--prompt-file` for long briefs. `getHtml()` / `getImage()` are URLs; `pull` is what writes files.

If `generate` / `edit` / `variants` times out or drops the connection, do **not** retry generation. Poll `screens` then `get` every 30s, up to 10 times — the job may still finish.

## Pipeline

1. Read `.stitch/DESIGN.md` if it exists. Read `.stitch/metadata.json` for the last project id.
2. Enhance the brief before generating. See [references/prompting.md](references/prompting.md). Vague prompts produce generic UI.
3. `ensure --title <name>` unless the user named an existing project id. Name is the product name in DESIGN.md if present, else the dest directory name.
4. `generate --device DESKTOP` unless DESIGN.md or the user named another device. Then `--pull` (or a follow-up `pull`). Timeout of at least 360s.
5. If the layout is right and the details are wrong, `edit` — do not regenerate. One targeted edit per call.
6. Variants only when the user wants alternatives (`REFINE` / `EXPLORE` / `REIMAGINE`).
7. After the first good screen, write or update `.stitch/DESIGN.md` from [templates/DESIGN.md](templates/DESIGN.md) so later screens stay consistent. Optionally `upload-md`.
8. Report project id, screen id, and local paths. Do not dump raw HTML.

Stay on this dest's product. Read DESIGN.md, AGENTS.md, or the user's brief for constraints.

## Local files

- **`.stitch/metadata.json`**: project id, title, pulled screens
- **`.stitch/DESIGN.md`**: visual source of truth for later prompts
- **`.stitch/designs/<slug>/code.html`**: pulled HTML
- **`.stitch/designs/<slug>/screen.png`**: pulled screenshot

Do not commit `.stitch/designs/`. Do not commit the API key.

## Prefer / do not

Prefer the CLI over `call`. Prefer edit over regenerate. Prefer a DESIGN.md after screen one.

Do not invent a Cursor/Claude MCP config for this dest — Pi does not load MCP. Do not paste `STITCH_API_KEY`. Do not put hex/fonts in a **generate** prompt when DESIGN.md already owns the theme; hex is fine in **edit** prompts.
