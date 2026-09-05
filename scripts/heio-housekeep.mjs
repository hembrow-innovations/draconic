#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import {
	boardOccupancy,
	CLEAR_TICKET_CLAIM_STATUSES,
	KEEP_SLICE_CLAIM_STATUSES,
	loadHeio,
	parseRootArg,
	wipState,
} from "./hivemind-status.mjs";

function escapeRegExp(s) {
	return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function hasClaim(row) {
	const value = row["claimed-by"];
	return typeof value === "string" && value.trim() !== "";
}

export function editFrontMatter(raw, fn) {
	if (!raw.startsWith("---")) return raw;
	const close = raw.indexOf("\n---", 3);
	if (close < 0) return raw;
	const inner = raw.slice(4, close);
	const lines = inner.split("\n");
	const next = fn(lines.slice());
	if (
		next.length === lines.length &&
		next.every((line, i) => line === lines[i])
	) {
		return raw;
	}
	return `---\n${next.join("\n")}${raw.slice(close)}`;
}

export function clearFrontKey(raw, key) {
	const re = new RegExp(`^${escapeRegExp(key)}:`);
	return editFrontMatter(raw, (lines) => lines.filter((line) => !re.test(line)));
}

export function setFrontKey(raw, key, value) {
	const re = new RegExp(`^${escapeRegExp(key)}:`);
	return editFrontMatter(raw, (lines) => {
		let found = false;
		const next = lines.map((line) => {
			if (!re.test(line)) return line;
			found = true;
			return `${key}: ${value}`;
		});
		return found ? next : lines;
	});
}

function assertHeioPath(root, abs) {
	const heioRoot = resolve(root, ".heio") + sep;
	const resolved = resolve(abs);
	if (resolved !== resolve(root, ".heio") && !resolved.startsWith(heioRoot)) {
		throw new Error(`refusing to write outside .heio/: ${abs}`);
	}
}

export function planHousekeep(board) {
	const changes = [];
	for (const ticket of board.tickets) {
		if (CLEAR_TICKET_CLAIM_STATUSES.has(ticket.status) && hasClaim(ticket)) {
			changes.push({
				abs: ticket.abs,
				id: ticket.id,
				action: "clear-claimed-by",
			});
		}
	}
	for (const slice of board.slices) {
		if (!KEEP_SLICE_CLAIM_STATUSES.has(slice.status) && hasClaim(slice)) {
			changes.push({
				abs: slice.abs,
				id: slice.id,
				action: "clear-claimed-by",
			});
		}
	}
	const occupancy = boardOccupancy(board.tickets, board.slices);
	const wip = wipState(board.tickets, board.slices);
	for (const pump of board.pumps) {
		if (wip === "at-cap" && pump.status === "idle") {
			changes.push({
				abs: pump.abs,
				id: pump.id,
				action: "set-status",
				from: "idle",
				to: "held",
			});
		} else if (occupancy === "empty" && pump.status === "held") {
			changes.push({
				abs: pump.abs,
				id: pump.id,
				action: "set-status",
				from: "held",
				to: "idle",
			});
		}
	}
	return { occupancy, wip, changes };
}

export function applyChange(root, change) {
	assertHeioPath(root, change.abs);
	const raw = readFileSync(change.abs, "utf8");
	let next = raw;
	if (change.action === "clear-claimed-by") {
		next = clearFrontKey(raw, "claimed-by");
	} else if (change.action === "set-status") {
		next = setFrontKey(raw, "status", change.to);
	}
	if (next !== raw) writeFileSync(change.abs, next);
}

export function formatChange(change) {
	if (change.action === "clear-claimed-by") {
		return `  ${change.id}: clear claimed-by`;
	}
	if (change.action === "set-status") {
		return `  ${change.id}: status ${change.from} -> ${change.to}`;
	}
	return `  ${change.id}: ${change.action}`;
}

export function parseHousekeepArgs(argv, fallbackRoot) {
	const root = parseRootArg(argv, fallbackRoot);
	const apply = argv.includes("--apply") && !argv.includes("--dry-run");
	return { root, apply };
}

export function main(argv = process.argv.slice(2)) {
	let parsed;
	try {
		parsed = parseHousekeepArgs(argv);
	} catch (err) {
		console.error(err instanceof Error ? err.message : String(err));
		return 1;
	}
	const { root, apply } = parsed;
	let board;
	try {
		board = loadHeio(root);
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		console.error(`unreadable .heio: ${msg}`);
		return 1;
	}
	const { occupancy, wip, changes } = planHousekeep(board);
	const mode = apply ? "apply" : "dry-run";
	console.log(`heio-housekeep ${mode}`);
	console.log(`  occupancy: ${occupancy}`);
	console.log(`  wip: ${wip}`);
	if (changes.length === 0) {
		console.log("  (no changes)");
		return 0;
	}
	for (const change of changes) {
		console.log(formatChange(change));
		if (apply) applyChange(root, change);
	}
	if (!apply) {
		console.log("  (dry-run; pass --apply to write)");
	}
	return 0;
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	process.exit(main());
}
