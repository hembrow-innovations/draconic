#!/usr/bin/env node
// Duplicate-id gate for the tracker vault (issues-209 / tasks-268).
//
// Fails if one numeric id is worn by two LIVE notes — same kind or not, since
// the vault runs a single global sequence (see `planning-next-id.mjs`). Live =
// anything outside `closed/` and `completed/`.
//
// Live-vs-archived reuse (`issues-244` live next to a completed `tasks-244`) is
// NOT a failure: the dead note keeps its filename, its wikilinks still resolve,
// and renumbering history is churn we deliberately refused (issues-209).
//
// Allocate ids with `node scripts/planning-next-id.mjs`. Never eyeball the
// highest number.
import { basename } from "node:path";
import { pathToFileURL } from "node:url";
import {
	assertVault,
	nextId,
	PLANNING_DIR,
	scanNotes,
} from "./planning-next-id.mjs";

// Grandfathered: live cross-kind collisions that already existed when this gate
// landed, all minted by the read-max-then-add-one allocator this task kills.
// They are pinned by exact filename, so a NEW note landing on one of these ids
// still fails. Do NOT extend this list — allocate with planning-next-id.mjs.
export const GRANDFATHERED = {};

/** Live notes grouped by id, keyed by id, values sorted basenames. */
const liveById = (dir) => {
	const groups = new Map();
	for (const n of scanNotes(dir).filter((n) => n.live)) {
		groups.set(n.id, [...(groups.get(n.id) ?? []), basename(n.file)].sort());
	}
	return groups;
};

const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);

/**
 * Live ids worn by more than one note, minus the grandfathered pairs.
 * @returns {{ id: number, files: string[] }[]} ordered by id
 */
export function findLiveDuplicates(
	dir = PLANNING_DIR,
	baseline = GRANDFATHERED,
) {
	return [...liveById(dir)]
		.filter(
			([id, files]) => files.length > 1 && !same(files, baseline[id] ?? []),
		)
		.map(([id, files]) => ({ id, files }))
		.sort((a, b) => a.id - b.id);
}

/** Grandfathered ids that are no longer live duplicates — safe to delete from the list. */
export function staleGrandfathered(
	dir = PLANNING_DIR,
	baseline = GRANDFATHERED,
) {
	const groups = liveById(dir);
	return Object.keys(baseline)
		.map(Number)
		.filter((id) => !same(groups.get(id) ?? [], baseline[id]))
		.sort((a, b) => a - b);
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	// Optional dir arg keeps the CLI testable against a fixture vault.
	const dir = process.argv[2] ?? PLANNING_DIR;
	const notes = assertVault(dir); // an empty scan is a broken path, not a pass
	const dupes = findLiveDuplicates(dir);
	const stale = staleGrandfathered(dir);

	if (stale.length) {
		console.warn(
			`! ${stale.length} grandfathered id(s) are no longer live duplicates — drop them from GRANDFATHERED in scripts/planning-check-ids.mjs: ${stale.join(", ")}`,
		);
	}

	if (dupes.length) {
		console.error(`✗ duplicate live tracker ids (${dupes.length}):`);
		for (const { id, files } of dupes)
			console.error(`  ${id} → ${files.join(", ")}`);
		console.error(
			`Rename the newer note with a free id: node scripts/planning-next-id.mjs (currently ${nextId(dir)}).`,
		);
		process.exit(1);
	}

	const live = notes.filter((n) => n.live).length;
	console.log(
		`✓ planning ids OK (${live} live notes, next id: ${nextId(dir)})`,
	);
}
