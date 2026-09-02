#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function front(raw) {
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

function walkMd(dir, acc = []) {
	if (!existsSync(dir)) return acc;
	for (const name of readdirSync(dir, { withFileTypes: true })) {
		const abs = join(dir, name.name);
		if (name.isDirectory()) walkMd(abs, acc);
		else if (name.isFile() && name.name.endsWith(".md")) acc.push(abs);
	}
	return acc;
}

function rel(abs) {
	return abs.slice(root.length + 1);
}

function printGroup(title, rows) {
	console.log(title);
	if (rows.length === 0) {
		console.log("  (none)");
		return;
	}
	for (const row of rows) {
		console.log(`  ${String(row.status ?? "?").padEnd(16)} ${row.id}  ${row.path}`);
	}
}

const tickets = walkMd(join(root, ".heio", "tickets"))
	.map((abs) => ({ path: rel(abs), ...front(readFileSync(abs, "utf8")) }))
	.filter((row) => row && row.id);
const slices = walkMd(join(root, ".heio", "planning"))
	.map((abs) => ({ path: rel(abs), ...front(readFileSync(abs, "utf8")) }))
	.filter((row) => row && row.kind === "slice");
const pumps = walkMd(join(root, ".heio", "planning"))
	.map((abs) => ({ path: rel(abs), ...front(readFileSync(abs, "utf8")) }))
	.filter((row) => row && row.kind === "pump");
const quarantine = existsSync(join(root, ".heio", "quarantine"))
	? readdirSync(join(root, ".heio", "quarantine")).filter((n) => n.endsWith(".md"))
	: [];

printGroup("tickets", tickets);
console.log("");
printGroup("slices", slices);
console.log("");
printGroup("pump", pumps);
console.log("");
console.log("quarantine");
console.log(quarantine.length ? quarantine.map((n) => `  ${n}`).join("\n") : "  (none)");
