import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

export type StackStatus = {
	sprintId: string | null;
	sliceId: string | null;
	freeze: string;
	tickets: string[];
};

type Note = { id: string; status: string };

function unquote(value: string): string {
	if (
		(value.startsWith('"') && value.endsWith('"') && value.length >= 2) ||
		(value.startsWith("'") && value.endsWith("'") && value.length >= 2)
	) {
		return value.slice(1, -1);
	}
	return value;
}

function readFrontmatter(path: string): Record<string, string> {
	let raw: string;
	try {
		raw = readFileSync(path, "utf8");
	} catch {
		return {};
	}
	if (!raw.startsWith("---")) return {};
	const end = raw.indexOf("\n---", 3);
	if (end === -1) return {};
	const block = raw.slice(3, end);
	const fields: Record<string, string> = {};
	for (const line of block.split("\n")) {
		const trimmed = line.trim();
		if (!trimmed || trimmed.startsWith("#")) continue;
		const colon = trimmed.indexOf(":");
		if (colon <= 0) continue;
		const key = trimmed.slice(0, colon).trim();
		const value = unquote(trimmed.slice(colon + 1).trim());
		if (key) fields[key] = value;
	}
	return fields;
}

function noteFrom(path: string, fallbackId: string): Note | undefined {
	if (!existsSync(path)) return undefined;
	const fields = readFrontmatter(path);
	const id = fields.id || fallbackId;
	const status = fields.status;
	if (!status) return undefined;
	return { id, status };
}

function listDirs(path: string): string[] {
	if (!existsSync(path)) return [];
	return readdirSync(path, { withFileTypes: true })
		.filter((ent) => ent.isDirectory())
		.map((ent) => ent.name)
		.sort();
}

function listMarkdown(path: string): string[] {
	if (!existsSync(path)) return [];
	return readdirSync(path)
		.filter((name) => name.endsWith(".md"))
		.sort();
}

function readSprints(cwd: string): Note[] {
	const root = join(cwd, ".heio", "planning", "sprints");
	const notes: Note[] = [];
	for (const id of listDirs(root)) {
		const note = noteFrom(join(root, id, "shape.md"), id);
		if (note) notes.push(note);
	}
	return notes;
}

function readSlices(cwd: string, sprintId: string): Note[] {
	const root = join(cwd, ".heio", "planning", "sprints", sprintId, "slices");
	const notes: Note[] = [];
	for (const id of listDirs(root)) {
		const note = noteFrom(join(root, id, "spec.md"), id);
		if (note) notes.push(note);
	}
	return notes;
}

function readOpenTickets(cwd: string): string[] {
	const root = join(cwd, ".heio", "tickets");
	const ids: string[] = [];
	for (const name of listMarkdown(root)) {
		const note = noteFrom(join(root, name), name.replace(/\.md$/, ""));
		if (note?.status === "open") ids.push(note.id);
	}
	return ids;
}

function pickSlice(slices: Note[]): Note | undefined {
	return (
		slices.find((slice) => slice.status === "active") ??
		slices.find((slice) => slice.status === "frozen")
	);
}

export function readStackStatus(cwd: string): StackStatus {
	const sprint = readSprints(cwd).find((note) => note.status === "active");
	const slice = sprint ? pickSlice(readSlices(cwd, sprint.id)) : undefined;
	return {
		sprintId: sprint?.id ?? null,
		sliceId: slice?.id ?? null,
		freeze: slice?.status ?? "none",
		tickets: readOpenTickets(cwd),
	};
}

function display(value: string | null): string {
	return value && value.length > 0 ? value : "none";
}

export function formatStackStatus(status: StackStatus): string {
	const tickets = status.tickets.length > 0 ? status.tickets.join(", ") : "none";
	return [
		`sprint: ${display(status.sprintId)}`,
		`slice: ${display(status.sliceId)}`,
		`freeze: ${display(status.freeze)}`,
		`tickets: ${tickets}`,
	].join("\n");
}
