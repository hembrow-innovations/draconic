import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, before, describe, test } from "node:test";
import { fileURLToPath } from "node:url";
import {
	boardOccupancy,
	census,
	countRoadmapTodos,
	loadBoard,
	wipState,
} from "./hivemind-status.mjs";

const SCRIPT = fileURLToPath(new URL("./hivemind-status.mjs", import.meta.url));

let tmp;

const note = (fm) => {
	const lines = Object.entries(fm).map(([k, v]) => `${k}: ${v}`);
	return `---\n${lines.join("\n")}\n---\n\n# fixture\n`;
};

const write = (root, rel, content) => {
	const abs = join(root, rel);
	fs.mkdirSync(dirname(abs), { recursive: true });
	fs.writeFileSync(abs, content);
};

const fixture = (name, { todos = 2, tickets = [], slices = [], pump } = {}) => {
	const root = join(tmp, name);
	fs.mkdirSync(root, { recursive: true });
	const rows = [];
	for (let i = 0; i < todos; i++) {
		rows.push(`| X${i} | todo | js | item ${i} | tests |`);
	}
	write(
		root,
		"ROADMAP.md",
		`# Roadmap\n\n**Status**: \`todo\` | \`in_progress\`\n\n${rows.join("\n")}\n`,
	);
	for (const t of tickets) {
		write(root, `.heio/tickets/${t.id}.md`, note(t));
	}
	for (const s of slices) {
		write(root, `.heio/planning/sprints/${s.id}.md`, note({ kind: "slice", ...s }));
	}
	if (pump) {
		write(root, ".heio/planning/pump.md", note({ kind: "pump", ...pump }));
	}
	return root;
};

const run = (args) =>
	spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });

before(async () => {
	tmp = await mkdtemp(join(tmpdir(), "hivemind-status-"));
});
after(async () => {
	await rm(tmp, { recursive: true, force: true });
});

describe("countRoadmapTodos", () => {
	test("counts | todo | rows and ignores legend prose", () => {
		const text = [
			"**Status**: `todo` | `in_progress` | `done`",
			"| ID | Status | Item |",
			"| X | todo | yes |",
			"| Y | done | no |",
			"| Z | todo | also |",
		].join("\n");
		assert.equal(countRoadmapTodos(text), 2);
	});
});

describe("census occupancy", () => {
	test("empty board is empty occupancy with zero in-flight", () => {
		const root = fixture("empty", {
			todos: 3,
			tickets: [{ id: "ticket-1", status: "promoted" }],
			slices: [{ id: "s-met", status: "met" }],
			pump: { id: "pump", status: "idle" },
		});
		const board = loadBoard(root);
		const snap = census(board);
		assert.equal(snap.roadmapTodos, 3);
		assert.equal(snap.occupancy, "empty");
		assert.equal(snap.inFlight, 0);
		assert.equal(snap.reviewBacklog, 0);
		assert.equal(snap.wip, "under-cap");
		assert.equal(snap.pump, "idle");
		assert.equal(boardOccupancy(board.tickets, board.slices), "empty");
	});

	test("ready-for-agent tickets occupy the board", () => {
		const root = fixture("ticket-inflight", {
			tickets: [{ id: "ticket-2", status: "ready-for-agent" }],
			pump: { id: "pump", status: "idle" },
		});
		const snap = census(loadBoard(root));
		assert.equal(snap.occupancy, "occupied");
		assert.equal(snap.inFlight, 1);
		assert.equal(snap.wip, "under-cap");
	});

	test("released slices occupy the board and count as review backlog", () => {
		const root = fixture("review-backlog", {
			slices: [
				{ id: "s-rel", status: "released" },
				{ id: "s-act", status: "active" },
			],
			pump: { id: "pump", status: "held" },
		});
		const snap = census(loadBoard(root));
		assert.equal(snap.occupancy, "occupied");
		assert.equal(snap.inFlight, 2);
		assert.equal(snap.reviewBacklog, 1);
		assert.equal(snap.pump, "held");
		assert.deepEqual(snap.sliceCounts, [
			["active", 1],
			["released", 1],
		]);
		assert.equal(snap.wip, "under-cap");
	});

	test("failed slices are not in-flight", () => {
		const root = fixture("failed-not-inflight", {
			slices: [{ id: "s-failed", status: "failed" }],
			pump: { id: "pump", status: "idle" },
		});
		const board = loadBoard(root);
		const snap = census(board);
		assert.equal(snap.occupancy, "empty");
		assert.equal(snap.inFlight, 0);
		assert.equal(wipState(board.tickets, board.slices), "under-cap");
	});

	test("three in-flight items are at WIP cap", () => {
		const root = fixture("at-cap", {
			slices: [
				{ id: "s-a", status: "active" },
				{ id: "s-b", status: "released" },
				{ id: "s-c", status: "reviewing" },
			],
			pump: { id: "pump", status: "held" },
		});
		const snap = census(loadBoard(root));
		assert.equal(snap.inFlight, 3);
		assert.equal(snap.wip, "at-cap");
		assert.equal(snap.occupancy, "occupied");
	});
});

describe("the status CLI", () => {
	test("prints listing plus census and exits 0", () => {
		const root = fixture("cli-ok", {
			todos: 4,
			tickets: [{ id: "ticket-9", status: "active" }],
			slices: [{ id: "s-ready", status: "ready" }],
			pump: { id: "pump", status: "idle" },
		});
		const { status, stdout, stderr } = run(["--root", root]);
		assert.equal(status, 0, stderr);
		assert.match(stdout, /^tickets\n/m);
		assert.match(stdout, /^slices\n/m);
		assert.match(stdout, /^pump\n/m);
		assert.match(stdout, /^quarantine\n/m);
		assert.match(stdout, /^census\n/m);
		assert.match(stdout, /ROADMAP todos: 4/);
		assert.match(stdout, /tickets by status\n    active: 1/);
		assert.match(stdout, /slices by status\n    ready: 1/);
		assert.match(stdout, /in-flight: 2/);
		assert.match(stdout, /occupancy: occupied/);
		assert.match(stdout, /wip: under-cap \(cap 3\)/);
		assert.match(stdout, /review backlog: 0/);
		assert.match(stdout, /pump: idle/);
	});

	test("exits 1 when ROADMAP.md is missing", () => {
		const root = join(tmp, "no-roadmap");
		fs.mkdirSync(join(root, ".heio"), { recursive: true });
		const { status, stderr } = run(["--root", root]);
		assert.equal(status, 1);
		assert.match(stderr, /unreadable ROADMAP\.md or \.heio/);
	});

	test("does not require the real .heio tree", () => {
		const root = fixture("no-real-heio", { todos: 0, pump: { id: "pump", status: "exhausted" } });
		const { status, stdout } = run(["--root", root]);
		assert.equal(status, 0);
		assert.match(stdout, /ROADMAP todos: 0/);
		assert.match(stdout, /occupancy: empty/);
		assert.match(stdout, /pump: exhausted/);
		assert.doesNotMatch(stdout, /\.heio\/tickets\/ticket-/);
	});
});
