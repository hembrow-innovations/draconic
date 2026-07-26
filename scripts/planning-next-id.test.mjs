import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, before, describe, test } from "node:test";
import { fileURLToPath } from "node:url";
import { nextId, scanNotes } from "./planning-next-id.mjs";

const SCRIPT = new URL("./planning-next-id.mjs", import.meta.url);

let tmp;

/** Materialise a fixture vault (a `docs/planning` stand-in) and return its root. */
const vault = (name, notes) => {
	const dir = join(tmp, name);
	for (const p of notes) {
		fs.mkdirSync(join(dir, dirname(p)), { recursive: true });
		fs.writeFileSync(join(dir, p), "# fixture\n");
	}
	fs.mkdirSync(dir, { recursive: true });
	return dir;
};

before(async () => {
	tmp = await mkdtemp(join(tmpdir(), "planning-ids-"));
});
after(async () => {
	await rm(tmp, { recursive: true, force: true });
});

describe("nextId", () => {
	test("an empty (or missing) vault allocates 1", () => {
		assert.equal(nextId(join(tmp, "does-not-exist")), 1);
		assert.equal(nextId(vault("empty", [])), 1);
	});

	test("the highest id wins wherever it lives — closed/, completed/, or another kind", () => {
		const dir = vault("max", [
			"issues/issues-3-live.md",
			"issues/closed/issues-40-closed.md",
			"tasks/tasks-7-live.md",
			"tasks/completed/tasks-41-completed.md",
			"plans/plans-42-plan.md",
		]);
		// The max (42) is a plan, so a global sequence hands out 43 — the "freed on
		// close" and "other kind" reuse bugs both die here (issues-209).
		assert.equal(nextId(dir), 43);
	});

	test("the highest id is found in closed/ and completed/ alone", () => {
		assert.equal(
			nextId(vault("closed-only", ["issues/closed/issues-88-x.md"])),
			89,
		);
		assert.equal(
			nextId(vault("completed-only", ["tasks/completed/tasks-99-x.md"])),
			100,
		);
	});

	test("ids compare numerically, not lexically", () => {
		const dir = vault("numeric", [
			"issues/issues-9-nine.md",
			"issues/issues-100-hundred.md",
		]);
		assert.equal(nextId(dir), 101);
	});

	test("legacy dashless ids (tasks5-…) still count", () => {
		assert.equal(
			nextId(vault("legacy", ["tasks/completed/tasks5-legacy.md"])),
			6,
		);
	});

	test("files without an id prefix are ignored", () => {
		const dir = vault("noise", [
			"index.md",
			"out-of-scope/discord-support-bridge.md",
		]);
		fs.writeFileSync(join(dir, "issues-2-not-markdown.txt"), "not a note\n");
		assert.equal(nextId(dir), 1);
	});
});

describe("the CLI", () => {
	const run = (dir) =>
		spawnSync(process.execPath, [fileURLToPath(SCRIPT), dir], {
			encoding: "utf8",
		});

	test("prints the next id", () => {
		const dir = vault("cli", ["issues/closed/issues-77-x.md"]);
		const { status, stdout } = run(dir);
		assert.equal(status, 0);
		assert.equal(stdout.trim(), "78");
	});

	test("exits 1 rather than printing 1 when the vault path is wrong", () => {
		const { status, stderr } = run(join(tmp, "not-a-vault"));
		assert.equal(status, 1);
		assert.match(stderr, /no tracker notes found/);
	});
});

describe("scanNotes", () => {
	test("parses kind + id and classifies live vs archived", () => {
		const dir = vault("scan", [
			"issues/issues-3-live.md",
			"tasks/tasks-4-live.md",
			"issues/closed/issues-5-closed.md",
			"tasks/completed/tasks-6-completed.md",
		]);
		assert.deepEqual(
			scanNotes(dir)
				.sort((a, b) => a.id - b.id)
				.map(({ id, kind, live }) => ({ id, kind, live })),
			[
				{ id: 3, kind: "issues", live: true },
				{ id: 4, kind: "tasks", live: true },
				{ id: 5, kind: "issues", live: false },
				{ id: 6, kind: "tasks", live: false },
			],
		);
	});
});
