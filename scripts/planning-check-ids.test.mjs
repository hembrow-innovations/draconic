import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, before, describe, test } from "node:test";
import { fileURLToPath } from "node:url";
import {
	findLiveDuplicates,
	GRANDFATHERED,
	staleGrandfathered,
} from "./planning-check-ids.mjs";
import { PLANNING_DIR } from "./planning-next-id.mjs";

const SCRIPT = new URL("./planning-check-ids.mjs", import.meta.url);

let tmp;

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
	tmp = await mkdtemp(join(tmpdir(), "planning-check-"));
});
after(async () => {
	await rm(tmp, { recursive: true, force: true });
});

describe("findLiveDuplicates", () => {
	test("a tree with unique live ids is clean", () => {
		const dir = vault("clean", [
			"issues/issues-1-a.md",
			"tasks/tasks-2-b.md",
			"plans/plans-3-c.md",
		]);
		assert.deepEqual(findLiveDuplicates(dir, {}), []);
	});

	test("two live notes of the same kind sharing an id fail", () => {
		const dir = vault("same-kind", [
			"issues/issues-7-a.md",
			"issues/issues-7-b.md",
		]);
		assert.deepEqual(findLiveDuplicates(dir, {}), [
			{ id: 7, files: ["issues-7-a.md", "issues-7-b.md"] },
		]);
	});

	test("a cross-kind live duplicate fails too", () => {
		const dir = vault("cross-kind", [
			"issues/issues-8-a.md",
			"tasks/tasks-8-b.md",
		]);
		assert.deepEqual(findLiveDuplicates(dir, {}), [
			{ id: 8, files: ["issues-8-a.md", "tasks-8-b.md"] },
		]);
	});

	test("a live id reused by a closed/completed note does not fail", () => {
		const dir = vault("live-vs-archived", [
			"issues/issues-9-a.md",
			"tasks/completed/tasks-9-b.md",
			"issues/closed/issues-9-c.md",
		]);
		assert.deepEqual(findLiveDuplicates(dir, {}), []);
	});

	test("a grandfathered pair is skipped, but any new file at that id fails", () => {
		const baseline = { 10: ["issues-10-a.md", "tasks-10-b.md"] };
		const pair = vault("grandfathered", [
			"issues/issues-10-a.md",
			"tasks/tasks-10-b.md",
		]);
		assert.deepEqual(findLiveDuplicates(pair, baseline), []);

		const extra = vault("grandfathered-plus", [
			"issues/issues-10-a.md",
			"tasks/tasks-10-b.md",
			"plans/plans-10-c.md",
		]);
		assert.deepEqual(findLiveDuplicates(extra, baseline), [
			{ id: 10, files: ["issues-10-a.md", "plans-10-c.md", "tasks-10-b.md"] },
		]);
	});

	test("a fresh collision at a grandfathered id fails — the baseline pins exact filenames", () => {
		const baseline = { 11: ["issues-11-a.md", "tasks-11-b.md"] };
		const dir = vault("rebound", [
			"issues/issues-11-a.md",
			"tasks/tasks-11-different.md",
		]);
		assert.deepEqual(findLiveDuplicates(dir, baseline), [
			{ id: 11, files: ["issues-11-a.md", "tasks-11-different.md"] },
		]);
	});

	test("duplicates are reported in id order", () => {
		const dir = vault("ordered", [
			"issues/issues-20-a.md",
			"tasks/tasks-20-b.md",
			"issues/issues-5-a.md",
			"tasks/tasks-5-b.md",
		]);
		assert.deepEqual(
			findLiveDuplicates(dir, {}).map((d) => d.id),
			[5, 20],
		);
	});
});

describe("staleGrandfathered", () => {
	test("reports baseline ids that are no longer live duplicates", () => {
		const dir = vault("stale", ["issues/issues-30-a.md"]);
		const baseline = {
			30: ["issues-30-a.md", "tasks-30-b.md"],
			31: ["issues-31-a.md"],
		};
		assert.deepEqual(staleGrandfathered(dir, baseline), [30, 31]);
	});

	test("an intact grandfathered pair is not stale", () => {
		const dir = vault("intact", [
			"issues/issues-32-a.md",
			"tasks/tasks-32-b.md",
		]);
		assert.deepEqual(
			staleGrandfathered(dir, { 32: ["issues-32-a.md", "tasks-32-b.md"] }),
			[],
		);
	});
});

describe("the gate CLI", () => {
	const gate = (dir) =>
		spawnSync(process.execPath, [fileURLToPath(SCRIPT), dir], {
			encoding: "utf8",
		});

	test("exits 1 and names both files on a live collision", () => {
		const dir = vault("cli-collision", [
			"issues/issues-40-a.md",
			"tasks/tasks-40-b.md",
		]);
		const { status, stderr } = gate(dir);
		assert.equal(status, 1);
		assert.match(stderr, /duplicate live tracker ids \(1\)/);
		assert.match(stderr, /40 → issues-40-a\.md, tasks-40-b\.md/);
	});

	test("exits 0 on a clean vault", () => {
		const dir = vault("cli-clean", [
			"issues/issues-41-a.md",
			"tasks/completed/tasks-41-b.md",
		]);
		const { status, stdout } = gate(dir);
		assert.equal(status, 0);
		assert.match(stdout, /planning ids OK \(1 live notes, next id: 42\)/);
	});

	test("exits 1 rather than vacuously passing when the vault path is wrong", () => {
		const { status, stderr } = gate(join(tmp, "not-a-vault"));
		assert.equal(status, 1);
		assert.match(stderr, /no tracker notes found/);
	});
});

describe("the real vault", () => {
	test("docs/planning has no un-grandfathered live duplicate ids", () => {
		assert.deepEqual(findLiveDuplicates(PLANNING_DIR, GRANDFATHERED), []);
	});

	test("the gate CLI passes the real tree", () => {
		const { status } = spawnSync(process.execPath, [fileURLToPath(SCRIPT)], {
			encoding: "utf8",
		});
		assert.equal(status, 0);
	});
});
