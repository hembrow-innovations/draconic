# obsidian-axi — upstream reference

Reference only. Prefer `rules/` plus the stack notes in `SKILL.md`. Do not load this file unless stuck or asked.

This pack uses the filesystem CLI `@andershoffmann/obsidian-axi`. It reads the vault on disk. Obsidian need not be running.

The unscoped npm package `obsidian-axi` (HermitCountry) wraps the official `obsidian` CLI and needs the app open. Do not use it here.

## Invoke

```sh
npx -y @andershoffmann/obsidian-axi <command>
```

A global install (`npm i -g @andershoffmann/obsidian-axi`) exposes `obsidian-axi`. Follow-up suggestions that start with `obsidian-axi` still run through the same binary you resolved (`disc-cli-binary`).

Flags go after the command. `--vault <name|path>` is a command flag, not a global prefix.

```sh
npx -y @andershoffmann/obsidian-axi search heio --vault docs
```

`-h` does nothing. Use `--help`. At the top level, `--help` must be the only argument.

## Vault selection (first match wins)

1. `--vault <name|path>` after the command
2. `OBSIDIAN_VAULT=<name|path>`
3. a `.obsidian/` folder in the current directory or above it
4. `defaultVault` in `~/.config/obsidian-axi/config.json`
5. the vault Obsidian has open
6. the vault opened most recently

`vault info` prints which rule applied. This checkout has no `docs/.obsidian/`. Pass `--vault docs`.

## Commands

- **(none) / home**: dashboard — vault, recent notes
- **ls**: list notes. `-r` / `--recursive`. `--sort modified`
- **read**: one or more paths. Bodies cap at 1000 characters. `--full`. `--metadata`
- **search**: full text. `--tag`, `--path`, `--frontmatter k=v`, `--modified-since 7d`, `--regex`, `--context N`
- **recent**: `--days N`. `--opened` reads the saved workspace
- **write**: create or replace. `--content` or `--content-file`
- **append**: add to the end. stdin works
- **patch**: one heading, block, or frontmatter field. `--target-type heading|frontmatter|block`. `--target`. `--op prepend|replace|delete`
- **rm**: move to `.trash`. `--permanent` deletes
- **mv**: rename or move
- **links**: backlinks. `--broken`. `--orphans`
- **tags**: tag counts. `tags files <tag>`
- **active**: last focused note from `.obsidian/workspace.json`
- **open**: ask Obsidian to jump to a note
- **vault**: `vault list`. `vault info`
- **setup**: `setup hooks` for Claude Code, Codex, OpenCode SessionStart. Do not run unless the human asked

A global install also gets AXI SDK `update`. Unnecessary with `npx -y`.

## AXI output

- TOON, not JSON. Lists include `count:`
- Default schemas are small. `--fields path` cuts columns
- Empty results say so in words
- Errors print on stdout with `code:` and `help[]`
- Exit `0` worked, including no-ops. `1` failed. `2` usage error
- Mutations are idempotent: append of text already at the end, delete of a missing note, move onto itself → no-op, exit 0

## Patch caveats

- Heading patch touches that heading's own content. Nested subsections stay. Child: `--target "Tasks::Today"`
- `--op delete` on a heading removes the heading and everything under it
- Frontmatter value lives in the target: `--target status=done`, not `--content`
- Newlines: `--content-file` or stdin
- Line endings and a trailing newline stay as they were

## Link resolution

Exact path, then relative to the linking note, then matching filename anywhere, then frontmatter aliases. Embeds (`![[Pasted image.png]]`) are not broken. Tag counts roll up (`#project` includes `#project/axi`). `--tag project` also matches `#project/axi`.

## Out of scope

Dataview, Templater, semantic search, and the command palette need a running Obsidian. This CLI returns `UNSUPPORTED`.

Obsidian Sync reloads outside edits, but an unsaved open note can overwrite a CLI write. Save or close first. Writes go via temp file then rename.

## AXI principles (<https://axi.md/>)

1. Token-efficient output — TOON
2. Minimal default schemas — 3–4 fields; extra via `--fields`
3. Content truncation — size hint plus `--full`
4. Pre-computed aggregates — `count:` upfront
5. Definitive empty states — explicit zero
6. Structured errors — stdout, idempotent mutations, no prompts
7. Ambient context — opt-in session hooks, then this skill
8. Content first — no-args is a dashboard, not help
9. Contextual disclosure — `help[]` next-step templates
10. Consistent help — per-subcommand `--help`

## This repo

- Vault is `docs/`
- New durable notes go through the **docs** skill (kind, template, layout, frontmatter)
- Wikilinks stay `[[note-name]]` per **docs**
