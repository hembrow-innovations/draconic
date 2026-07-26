#!/usr/bin/env node
// Tracker id allocator (issues-209 / tasks-268).
//
// The vault runs ONE global id sequence: `issues-N`, `tasks-N` and `plans-N`
// all draw from it. Eyeballing the highest number in the *active* folder frees
// an id the moment a note is closed, and ignores the other kind entirely — that
// is how the vault ended up with 20+ duplicated ids.
//
// This scans EVERY note under `docs/planning` (issues/, issues/closed/, tasks/,
// tasks/completed/, plans/, …) and prints `max + 1`.
//
//   node scripts/planning-next-id.mjs   →   310
//
// Re-run it immediately before writing the file: it reads the high-water mark
// without reserving it, so a concurrent session can still take the number in
// between. `scripts/planning-check-ids.mjs` is the backstop that makes any
// surviving collision loud.
import fs from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const PLANNING_DIR = resolve(
	dirname(fileURLToPath(import.meta.url)),
	"../docs/planning",
);

// Ids are read from the FILENAME, not frontmatter — the filename is what
// wikilinks resolve on, and some historical notes' `id:` keys drifted from it.
// `issues-209-…`, `tasks-268-…`, `plans-3-…`, plus the legacy dashless `tasks5-…`.
const ID_RE = /^(issues|tasks|plans)-?(\d+)/;

// A note lives in `closed/` or `completed/` once it is done; anything else is
// live — including `out-of-scope/`, whose ids stay reserved.
const ARCHIVED_RE = /(^|\/)(closed|completed)\//;

/**
 * Every id-bearing note under `dir`, recursively.
 * @returns {{ id: number, kind: string, file: string, name: string, live: boolean }[]}
 */
export function scanNotes(dir = PLANNING_DIR) {
	if (!fs.existsSync(dir)) return [];
	return fs
		.readdirSync(dir, { recursive: true })
		.filter((p) => p.endsWith(".md"))
		.flatMap((p) => {
			const name = basename(p);
			const m = ID_RE.exec(name);
			return m
				? [
						{
							id: Number(m[2]),
							kind: m[1],
							file: p,
							name,
							live: !ARCHIVED_RE.test(p),
						},
					]
				: [];
		});
}

/** The next free id in the single global sequence — max over every folder, plus one. */
export const nextId = (dir = PLANNING_DIR) =>
	Math.max(0, ...scanNotes(dir).map((n) => n.id)) + 1;

/**
 * A vault that scans to zero notes means a broken path, not a clean tree — the
 * exact vacuously-green failure the gate must never have. Callers exit on it.
 */
export function assertVault(dir) {
	const notes = scanNotes(dir);
	if (!notes.length) {
		console.error(
			`✗ no tracker notes found under ${dir} — wrong path, or the vault moved.`,
		);
		process.exit(1);
	}
	return notes;
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	const dir = process.argv[2] ?? PLANNING_DIR;
	assertVault(dir);
	console.log(nextId(dir));
}
