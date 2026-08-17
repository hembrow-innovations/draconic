#!/usr/bin/env node
// Parse ROADMAP.md status counts. CLI: node .loop/roadmap-status.mjs [--json]
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const ROW =
	/^\|\s*([A-Za-z][A-Za-z0-9.]*)\s*\|\s*(todo|in_progress|done|blocked)\s*\|/gm;

export function roadmapPath(cwd = process.cwd()) {
	return resolve(cwd, "ROADMAP.md");
}

export function parseRoadmap(md) {
	const counts = { todo: 0, in_progress: 0, done: 0, blocked: 0 };
	/** @type {{ id: string, status: string }[]} */
	const items = [];
	for (const m of md.matchAll(ROW)) {
		const id = m[1];
		const status = m[2];
		counts[status]++;
		items.push({ id, status });
	}
	return { counts, items, empty: counts.todo === 0 && counts.in_progress === 0 };
}

export function readRoadmapStatus(cwd = process.cwd()) {
	const path = roadmapPath(cwd);
	const md = readFileSync(path, "utf8");
	return { path, ...parseRoadmap(md) };
}

const isMain =
	process.argv[1] &&
	import.meta.url === pathToFileURL(resolve(process.argv[1])).href;

if (isMain) {
	const st = readRoadmapStatus();
	if (process.argv.includes("--json")) {
		console.log(JSON.stringify(st, null, 2));
	} else {
		const { counts, empty } = st;
		console.log(
			`todo=${counts.todo} in_progress=${counts.in_progress} done=${counts.done} blocked=${counts.blocked} empty=${empty}`,
		);
	}
}
