#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const IN_FLIGHT_TICKET_STATUSES = new Set(["ready-for-agent", "active"]);
export const IN_FLIGHT_SLICE_STATUSES = new Set([
	"ready",
	"active",
	"released",
	"reviewing",
	"failed",
]);
export const CLEAR_TICKET_CLAIM_STATUSES = new Set([
	"promoted",
	"dropped",
	"closed",
]);
export const KEEP_SLICE_CLAIM_STATUSES = new Set(["active", "reviewing"]);

const defaultRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

export function parseFront(raw) {
	if (!raw.startsWith("---")) return null;
	const close = raw.indexOf("\n---", 3);
	if (close < 0) return null;
	const map = {};
	for (const line of raw.slice(4, close).split("\n")) {
		const m = line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/);
		if (m) map[m[1]] = m[2].replace(/^"|"$/g, "");
	}
	return map;
}

export function walkMd(dir, acc = []) {
	if (!existsSync(dir)) return acc;
	for (const name of readdirSync(dir, { withFileTypes: true })) {
		const abs = join(dir, name.name);
		if (name.isDirectory()) walkMd(abs, acc);
		else if (name.isFile() && name.name.endsWith(".md")) acc.push(abs);
	}
	return acc;
}

export function rel(root, abs) {
	return abs.slice(root.length + 1);
}

export function parseRootArg(argv, fallback = defaultRoot) {
	const i = argv.indexOf("--root");
	if (i < 0) return fallback;
	const value = argv[i + 1];
	if (!value || value.startsWith("--")) {
		throw new Error("missing --root path");
	}
	return value;
}

function loadNotes(root, dir, keep) {
	return walkMd(dir)
		.map((abs) => {
			const raw = readFileSync(abs, "utf8");
			return { abs, path: rel(root, abs), raw, ...parseFront(raw) };
		})
		.filter(keep);
}

export function countRoadmapTodos(text) {
	return text.split(/\r?\n/).filter((line) => line.includes("| todo |")).length;
}

export function loadHeio(root) {
	const tickets = loadNotes(root, join(root, ".heio", "tickets"), (row) =>
		Boolean(row && row.id),
	);
	const planning = loadNotes(root, join(root, ".heio", "planning"), (row) =>
		Boolean(row && row.id),
	);
	const slices = planning.filter((row) => row.kind === "slice");
	const pumps = planning.filter((row) => row.kind === "pump");
	const quarantineDir = join(root, ".heio", "quarantine");
	const quarantine = existsSync(quarantineDir)
		? readdirSync(quarantineDir).filter((n) => n.endsWith(".md"))
		: [];
	return { tickets, slices, pumps, quarantine };
}

export function loadRoadmapTodos(root) {
	const abs = join(root, "ROADMAP.md");
	return countRoadmapTodos(readFileSync(abs, "utf8"));
}

export function inFlightTickets(tickets) {
	return tickets.filter((row) => IN_FLIGHT_TICKET_STATUSES.has(row.status));
}

export function inFlightSlices(slices) {
	return slices.filter((row) => IN_FLIGHT_SLICE_STATUSES.has(row.status));
}

export function boardOccupancy(tickets, slices) {
	return inFlightTickets(tickets).length + inFlightSlices(slices).length > 0
		? "occupied"
		: "empty";
}

export function groupCounts(rows) {
	const counts = new Map();
	for (const row of rows) {
		const status = row.status ?? "?";
		counts.set(status, (counts.get(status) ?? 0) + 1);
	}
	return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

export function census(board) {
	const { tickets, slices, pumps, roadmapTodos } = board;
	const inFlight =
		inFlightTickets(tickets).length + inFlightSlices(slices).length;
	return {
		roadmapTodos,
		ticketCounts: groupCounts(tickets),
		sliceCounts: groupCounts(slices),
		inFlight,
		occupancy: boardOccupancy(tickets, slices),
		reviewBacklog: slices.filter((row) => row.status === "released").length,
		pump:
			pumps.length === 0
				? "(none)"
				: pumps.map((row) => row.status ?? "?").join(", "),
	};
}

export function printGroup(title, rows) {
	console.log(title);
	if (rows.length === 0) {
		console.log("  (none)");
		return;
	}
	for (const row of rows) {
		console.log(
			`  ${String(row.status ?? "?").padEnd(16)} ${row.id}  ${row.path}`,
		);
	}
}

function printCountGroup(title, counts) {
	console.log(`  ${title}`);
	if (counts.length === 0) {
		console.log("    (none)");
		return;
	}
	for (const [status, n] of counts) {
		console.log(`    ${status}: ${n}`);
	}
}

export function printCensus(board) {
	const snap = census(board);
	console.log("census");
	console.log(`  ROADMAP todos: ${snap.roadmapTodos}`);
	printCountGroup("tickets by status", snap.ticketCounts);
	printCountGroup("slices by status", snap.sliceCounts);
	console.log(`  in-flight: ${snap.inFlight}`);
	console.log(`  occupancy: ${snap.occupancy}`);
	console.log(`  review backlog: ${snap.reviewBacklog}`);
	console.log(`  pump: ${snap.pump}`);
}

export function loadBoard(root) {
	const heio = loadHeio(root);
	const roadmapTodos = loadRoadmapTodos(root);
	return { ...heio, roadmapTodos };
}

export function main(argv = process.argv.slice(2)) {
	let root;
	try {
		root = parseRootArg(argv);
	} catch (err) {
		console.error(err instanceof Error ? err.message : String(err));
		return 1;
	}
	let board;
	try {
		board = loadBoard(root);
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		console.error(`unreadable ROADMAP.md or .heio: ${msg}`);
		return 1;
	}
	printGroup("tickets", board.tickets);
	console.log("");
	printGroup("slices", board.slices);
	console.log("");
	printGroup("pump", board.pumps);
	console.log("");
	console.log("quarantine");
	console.log(
		board.quarantine.length
			? board.quarantine.map((n) => `  ${n}`).join("\n")
			: "  (none)",
	);
	console.log("");
	printCensus(board);
	return 0;
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	process.exit(main());
}
