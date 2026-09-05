import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, before, describe, test } from "node:test";
import { fileURLToPath } from "node:url";
import {
	clearFrontKey,
	planHousekeep,
	setFrontKey,
} from "./heio-housekeep.mjs";
import { loadHeio, parseFront } from "./hivemind-status.mjs";

const SCRIPT = fileURLToPath(new URL("./heio-housekeep.mjs", import.meta.url));

let tmp;

const note = (fm, body = "# fixture\n- **O1**: unmet\n") => {
	const lines = Object.entries(fm).map(([k, v]) => `${k}: ${v}`);
	return `---\n${lines.join("\n")}\n---\n\n${body}`;
};

const write = (root, rel, content) => {
	const abs = join(root, rel);
	fs.mkdirSync(dirname(abs), { recursive: true });
	fs.writeFileSync(abs, content);
};

const fixture = (
	name,
	{ tickets = [], slices = [], pump, roadmap = "# Roadmap\n| X | todo | y |\n" } = {},
) => {
	const root = join(tmp, name);
	fs.mkdirSync(root, { recursive: true });
	write(root, "ROADMAP.md", roadmap);
	for (const t of tickets) {
		write(root, `.heio/tickets/${t.id}.md`, note(t));
	}
	for (const s of slices) {
		write(
			root,
			`.heio/planning/sprints/${s.id}.md`,
			note({ kind: "slice", ...s }),
		);
	}
	if (pump) {
		write(root, ".heio/planning/pump.md", note({ kind: "pump", ...pump }));
	}
	return root;
};

const run = (args) =>
	spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });

const frontOf = (root, rel) => parseFront(fs.readFileSync(join(root, rel), "utf8"));

before(async () => {
	tmp = await mkdtemp(join(tmpdir(), "heio-housekeep-"));
});
after(async () => {
	await rm(tmp, { recursive: true, force: true });
});

describe("front matter edits", () => {
	test("clearFrontKey removes claimed-by and leaves the body", () => {
		const raw = note(
			{ id: "ticket-1", status: "promoted", "claimed-by": "abc" },
			"# keep\n- **O1**: unmet\n",
		);
		const next = clearFrontKey(raw, "claimed-by");
		assert.equal(parseFront(next)["claimed-by"], undefined);
		assert.equal(parseFront(next).status, "promoted");
		assert.match(next, /# keep/);
		assert.match(next, /- \*\*O1\*\*: unmet/);
	});

	test("setFrontKey updates status only", () => {
		const raw = note({ id: "pump", kind: "pump", status: "idle" });
		const next = setFrontKey(raw, "status", "held");
		assert.equal(parseFront(next).status, "held");
		assert.equal(parseFront(next).kind, "pump");
	});
});

describe("planHousekeep", () => {
	test("clears promoted/dropped/closed ticket claims and stale slice claims", () => {
		const root = fixture("plan-claims", {
			tickets: [
				{ id: "ticket-promoted", status: "promoted", "claimed-by": "a" },
				{ id: "ticket-dropped", status: "dropped", "claimed-by": "b" },
				{ id: "ticket-closed", status: "closed", "claimed-by": "c" },
				{ id: "ticket-active", status: "active", "claimed-by": "keep-ticket" },
			],
			slices: [
				{ id: "s-ready", status: "ready", "claimed-by": "d" },
				{ id: "s-released", status: "released", "claimed-by": "e" },
				{ id: "s-met", status: "met", "claimed-by": "f" },
				{ id: "s-failed", status: "failed", "claimed-by": "g" },
				{ id: "s-active", status: "active", "claimed-by": "keep-slice" },
				{ id: "s-reviewing", status: "reviewing", "claimed-by": "keep-review" },
			],
			pump: { id: "pump", status: "idle" },
		});
		const { occupancy, changes } = planHousekeep(loadHeio(root));
		assert.equal(occupancy, "occupied");
		const ids = changes.filter((c) => c.action === "clear-claimed-by").map((c) => c.id);
		assert.deepEqual(ids.sort(), [
			"s-failed",
			"s-met",
			"s-ready",
			"s-released",
			"ticket-closed",
			"ticket-dropped",
			"ticket-promoted",
		]);
		assert.equal(
			changes.some((c) => c.id === "ticket-active" || c.id === "s-active" || c.id === "s-reviewing"),
			false,
		);
	});

	test("at-cap idle pump becomes held; empty held pump becomes idle", () => {
		const atCap = fixture("occ-idle", {
			slices: [
				{ id: "s-a", status: "active" },
				{ id: "s-b", status: "released" },
				{ id: "s-c", status: "reviewing" },
			],
			pump: { id: "pump", status: "idle" },
		});
		const occPlan = planHousekeep(loadHeio(atCap));
		assert.equal(occPlan.occupancy, "occupied");
		assert.equal(occPlan.wip, "at-cap");
		assert.deepEqual(
			occPlan.changes.filter((c) => c.action === "set-status"),
			[
				{
					abs: occPlan.changes.find((c) => c.action === "set-status").abs,
					id: "pump",
					action: "set-status",
					from: "idle",
					to: "held",
				},
			],
		);

		const emptyHeld = fixture("empty-held", {
			tickets: [{ id: "ticket-promoted", status: "promoted" }],
			slices: [{ id: "s-met", status: "met" }],
			pump: { id: "pump", status: "held" },
		});
		const emptyPlan = planHousekeep(loadHeio(emptyHeld));
		assert.equal(emptyPlan.occupancy, "empty");
		assert.equal(emptyPlan.changes[0].to, "idle");
		assert.equal(emptyPlan.changes[0].from, "held");
	});

	test("occupied under-cap idle pump stays idle so planner can feed", () => {
		const under = fixture("under-idle", {
			slices: [{ id: "s-rel", status: "released" }],
			pump: { id: "pump", status: "idle" },
		});
		const plan = planHousekeep(loadHeio(under));
		assert.equal(plan.occupancy, "occupied");
		assert.equal(plan.wip, "under-cap");
		assert.equal(
			plan.changes.some((c) => c.action === "set-status"),
			false,
		);
	});

	test("does not move exhausted or already-correct pump status", () => {
		const exhausted = fixture("exhausted", {
			pump: { id: "pump", status: "exhausted" },
		});
		assert.equal(planHousekeep(loadHeio(exhausted)).changes.length, 0);

		const occupiedHeld = fixture("occ-held", {
			tickets: [{ id: "ticket-ready", status: "ready-for-agent" }],
			pump: { id: "pump", status: "held" },
		});
		assert.equal(
			planHousekeep(loadHeio(occupiedHeld)).changes.some((c) => c.action === "set-status"),
			false,
		);

		const emptyIdle = fixture("empty-idle", {
			pump: { id: "pump", status: "idle" },
		});
		assert.equal(planHousekeep(loadHeio(emptyIdle)).changes.length, 0);
	});
});

describe("the housekeep CLI", () => {
	test("defaults to dry-run and does not write", () => {
		const root = fixture("dry-default", {
			tickets: [
				{ id: "ticket-promoted", status: "promoted", "claimed-by": "stale" },
			],
			pump: { id: "pump", status: "idle" },
		});
		const before = fs.readFileSync(
			join(root, ".heio/tickets/ticket-promoted.md"),
			"utf8",
		);
		const { status, stdout } = run(["--root", root]);
		assert.equal(status, 0);
		assert.match(stdout, /heio-housekeep dry-run/);
		assert.match(stdout, /ticket-promoted: clear claimed-by/);
		assert.equal(
			fs.readFileSync(join(root, ".heio/tickets/ticket-promoted.md"), "utf8"),
			before,
		);
	});

	test("--dry-run does not write either", () => {
		const root = fixture("dry-flag", {
			slices: [
				{ id: "s-a", status: "active" },
				{ id: "s-b", status: "released" },
				{ id: "s-c", status: "reviewing" },
			],
			pump: { id: "pump", status: "idle" },
		});
		const before = fs.readFileSync(join(root, ".heio/planning/pump.md"), "utf8");
		const { status, stdout } = run(["--root", root, "--dry-run"]);
		assert.equal(status, 0);
		assert.match(stdout, /dry-run/);
		assert.match(stdout, /pump: status idle -> held/);
		assert.equal(fs.readFileSync(join(root, ".heio/planning/pump.md"), "utf8"), before);
	});

	test("--apply clears claims, holds occupied pump, and leaves statuses/oracles/roadmap", () => {
		const roadmap = "# Roadmap\n| X | todo | keep |\n";
		const root = fixture("apply", {
			roadmap,
			tickets: [
				{
					id: "ticket-promoted",
					status: "promoted",
					"claimed-by": "stale-ticket",
				},
				{
					id: "ticket-active",
					status: "active",
					"claimed-by": "keep-ticket",
				},
			],
			slices: [
				{
					id: "s-released",
					status: "released",
					"claimed-by": "stale-slice",
				},
				{
					id: "s-active",
					status: "active",
					"claimed-by": "keep-slice",
				},
				{
					id: "s-reviewing",
					status: "reviewing",
					"claimed-by": "keep-review",
				},
			],
			pump: { id: "pump", status: "idle" },
		});
		const { status, stdout } = run(["--root", root, "--apply"]);
		assert.equal(status, 0);
		assert.match(stdout, /heio-housekeep apply/);
		assert.equal(
			frontOf(root, ".heio/tickets/ticket-promoted.md")["claimed-by"],
			undefined,
		);
		assert.equal(frontOf(root, ".heio/tickets/ticket-promoted.md").status, "promoted");
		assert.equal(
			frontOf(root, ".heio/tickets/ticket-active.md")["claimed-by"],
			"keep-ticket",
		);
		assert.equal(
			frontOf(root, ".heio/planning/sprints/s-released.md")["claimed-by"],
			undefined,
		);
		assert.equal(frontOf(root, ".heio/planning/sprints/s-released.md").status, "released");
		assert.equal(
			frontOf(root, ".heio/planning/sprints/s-active.md")["claimed-by"],
			"keep-slice",
		);
		assert.equal(
			frontOf(root, ".heio/planning/sprints/s-reviewing.md")["claimed-by"],
			"keep-review",
		);
		assert.equal(frontOf(root, ".heio/planning/pump.md").status, "held");
		assert.match(
			fs.readFileSync(join(root, ".heio/planning/sprints/s-released.md"), "utf8"),
			/- \*\*O1\*\*: unmet/,
		);
		assert.equal(fs.readFileSync(join(root, "ROADMAP.md"), "utf8"), roadmap);
		assert.equal(fs.existsSync(join(root, ".heio/archive")), false);
	});

	test("--apply idles a held pump on an empty board", () => {
		const root = fixture("apply-empty", {
			tickets: [{ id: "ticket-promoted", status: "promoted", "claimed-by": "x" }],
			pump: { id: "pump", status: "held" },
		});
		const { status, stdout } = run(["--root", root, "--apply"]);
		assert.equal(status, 0);
		assert.match(stdout, /pump: status held -> idle/);
		assert.equal(frontOf(root, ".heio/planning/pump.md").status, "idle");
		assert.equal(
			frontOf(root, ".heio/tickets/ticket-promoted.md")["claimed-by"],
			undefined,
		);
	});
});
