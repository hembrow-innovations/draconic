#!/usr/bin/env node
// One swarm wave: up to N fresh opencode sessions, each one draconic-loop.
// Usage:
//   node .loop/opencode-swarm.mjs
//   node .loop/opencode-swarm.mjs wave=10
//   node .loop/opencode-swarm.mjs parallel wave=10
//   node .loop/opencode-swarm.mjs --wave 5 --mode serial
//   node .loop/opencode-swarm.mjs wave=10 -- -m xai/grok-4.5
//
// serial (default): N sequential sessions (safe on shared worktree)
// parallel: N concurrent sessions (claim races + edit conflicts possible)
//
// Stops early if ROADMAP has no todo rows. Exit 0 on empty board or completed wave.
// Exit 2 if board still has todos but wave made zero progress (done count unchanged).

import { readRoadmapStatus } from "./roadmap-status.mjs";
import {
	DEFAULT_LOOP_PROMPT,
	runOpencodeOnce,
	sleep,
	stallConfig,
} from "./run-opencode.mjs";

function parseArgs(argv) {
	let wave = Number.parseInt(process.env.WAVE || "10", 10);
	let mode = (process.env.SWARM_MODE || "serial").toLowerCase();
	/** @type {string[]} */
	const promptParts = [];
	/** @type {string[]} */
	const extraFlags = [];
	let afterDash = false;

	for (const a of argv) {
		if (afterDash) {
			extraFlags.push(a);
			continue;
		}
		if (a === "--") {
			afterDash = true;
			continue;
		}
		if (a === "parallel" || a === "--parallel") {
			mode = "parallel";
			continue;
		}
		if (a === "serial" || a === "--serial") {
			mode = "serial";
			continue;
		}
		const waveEq = /^wave=(\d+)$/i.exec(a);
		if (waveEq) {
			wave = Number.parseInt(waveEq[1], 10);
			continue;
		}
		if (a === "--wave" || a === "-n") {
			continue; // value next
		}
		const prev = argv[argv.indexOf(a) - 1];
		if (prev === "--wave" || prev === "-n") {
			wave = Number.parseInt(a, 10);
			continue;
		}
		if (a.startsWith("-")) {
			extraFlags.push(a);
			continue;
		}
		promptParts.push(a);
	}

	if (!Number.isInteger(wave) || wave < 1) {
		console.error(
			"Usage: node .loop/opencode-swarm.mjs [parallel|serial] [wave=N] [-- opencode flags]",
		);
		process.exit(1);
	}
	if (mode !== "serial" && mode !== "parallel") {
		console.error(`Unknown mode: ${mode}`);
		process.exit(1);
	}

	const promptArgs =
		promptParts.length > 0 ? promptParts : [DEFAULT_LOOP_PROMPT];
	return { wave, mode, promptArgs, extraFlags };
}

const { wave, mode, promptArgs, extraFlags } = parseArgs(process.argv.slice(2));
const { stallSec, stallAction } = stallConfig();
const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;

const before = readRoadmapStatus();
console.log(
	`swarm mode=${mode} wave=${wave} todo=${before.counts.todo} in_progress=${before.counts.in_progress} done=${before.counts.done}`,
);
console.log(
	`stall watchdog: ${stallSec > 0 ? `${stallSec}s (${stallAction})` : "disabled"}`,
);
console.log(`prompt: ${promptArgs.join(" ")}`);

if (before.counts.todo === 0) {
	console.log("empty board (no todo) — swarm nothing to do");
	process.exit(0);
}

if (mode === "parallel") {
	console.warn(
		"[warn] parallel mode: concurrent agents share one worktree; expect claim races and merge conflicts. Prefer serial for this monorepo.",
	);
}

let stalls = 0;
let errors = 0;
let ran = 0;

async function oneSlot(i) {
	const snap = readRoadmapStatus();
	if (snap.counts.todo === 0) {
		return { code: 0, reason: "empty", skipped: true };
	}
	const { code, reason } = await runOpencodeOnce({
		label: `swarm ${i}/${wave} (${mode})`,
		promptArgs,
		extraFlags,
	});
	return { code, reason, skipped: false };
}

if (mode === "serial") {
	for (let i = 1; i <= wave; i++) {
		const r = await oneSlot(i);
		if (r.skipped) {
			console.log("empty board mid-wave — stopping swarm");
			break;
		}
		ran++;
		if (r.reason === "stall") {
			stalls++;
			console.error(`slot ${i} stalled (total stalls: ${stalls})`);
			if (stallAction === "abort") {
				console.error("STALL_ACTION=abort — stopping swarm");
				process.exit(1);
			}
		} else if (r.code !== 0) {
			errors++;
			console.error(`slot ${i} exited with code ${r.code}`);
		}
		if (sleepMs && i < wave) {
			console.log(`sleeping ${sleepMs / 1000}s...`);
			await sleep(sleepMs);
		}
	}
} else {
	const slots = Array.from({ length: wave }, (_, idx) => idx + 1);
	const results = await Promise.all(slots.map((i) => oneSlot(i)));
	for (const r of results) {
		if (r.skipped) continue;
		ran++;
		if (r.reason === "stall") stalls++;
		else if (r.code !== 0) errors++;
	}
}

const after = readRoadmapStatus();
const doneDelta = after.counts.done - before.counts.done;
const todoDelta = before.counts.todo - after.counts.todo;

console.log(
	`\n===== swarm done: ran=${ran}/${wave} stalls=${stalls} errors=${errors} doneΔ=${doneDelta} todoΔ=${todoDelta} remaining_todo=${after.counts.todo} =====`,
);

if (after.counts.todo === 0) {
	console.log("roadmap complete (no todo left)");
	process.exit(stalls > 0 || errors > 0 ? 1 : 0);
}

// Wave finished but board not empty: zero progress → signal orchestrator
if (ran > 0 && doneDelta === 0 && todoDelta === 0) {
	console.error("no roadmap progress this wave — exit 2");
	process.exit(2);
}

process.exit(stalls > 0 || errors > 0 ? 1 : 0);
