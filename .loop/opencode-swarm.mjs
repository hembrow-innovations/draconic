#!/usr/bin/env node
// One swarm wave: up to N fresh opencode sessions, each one draconic-loop.
// Usage:
//   node .loop/opencode-swarm.mjs
//   node .loop/opencode-swarm.mjs wave=10
//   node .loop/opencode-swarm.mjs parallel wave=10
//   node .loop/opencode-swarm.mjs wave=10 -- -m xai/grok-4.5
//
// serial (default): N sequential sessions on the main worktree
// parallel: N concurrent sessions, each in its own git worktree under
//           .loop/worktrees/. After each slot finishes (ok/error/stall),
//           the worktree is always removed. Orphans are swept on start/exit.
//
// Stops early if ROADMAP has no todo rows. Exit 0 on empty board or completed wave.
// Exit 2 if board still has todos but wave made zero progress.

import { readRoadmapStatus } from "./roadmap-status.mjs";
import {
	DEFAULT_LOOP_PROMPT,
	runOpencodeOnce,
	sleep,
	stallConfig,
} from "./run-opencode.mjs";
import {
	cleanupAllSwarmWorktrees,
	commitsOnBranch,
	createSwarmWorktree,
	installWorktreeCleanupHandlers,
	mergeSwarmBranch,
	removeWorktree,
	repoRoot,
} from "./worktree.mjs";

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
		if (a === "--wave" || a === "-n") continue;
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

const root = repoRoot();
installWorktreeCleanupHandlers(root);

// Always start clean — no dangling swarm worktrees from prior crashes
cleanupAllSwarmWorktrees(root, { label: "swarm-start" });

const { wave, mode, promptArgs, extraFlags } = parseArgs(process.argv.slice(2));
const { stallSec, stallAction } = stallConfig();
const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;
const waveId = `${Date.now().toString(36)}`;

// Serialize merges into main so parallel finishers don't race git
/** @type {Promise<void>} */
let mergeChain = Promise.resolve();
function withMergeLock(fn) {
	const run = mergeChain.then(fn, fn);
	mergeChain = run.then(
		() => {},
		() => {},
	);
	return run;
}

const before = readRoadmapStatus(root);
console.log(
	`swarm mode=${mode} wave=${wave} todo=${before.counts.todo} in_progress=${before.counts.in_progress} done=${before.counts.done}`,
);
console.log(
	`stall watchdog: ${stallSec > 0 ? `${stallSec}s (${stallAction})` : "disabled"}`,
);
console.log(`prompt: ${promptArgs.join(" ")}`);
if (mode === "parallel") {
	console.log(
		`worktrees: .loop/worktrees/ (create → run → merge → remove; always cleaned)`,
	);
}

if (before.counts.todo === 0) {
	console.log("empty board (no todo) — swarm nothing to do");
	cleanupAllSwarmWorktrees(root, { label: "swarm-empty" });
	process.exit(0);
}

let stalls = 0;
let errors = 0;
let ran = 0;
let mergeFails = 0;

/**
 * @param {number} i
 */
async function oneSlotSerial(i) {
	const snap = readRoadmapStatus(root);
	if (snap.counts.todo === 0) {
		return { code: 0, reason: "empty", skipped: true };
	}
	const { code, reason } = await runOpencodeOnce({
		label: `swarm ${i}/${wave} (serial)`,
		promptArgs,
		extraFlags,
		cwd: root,
	});
	return { code, reason, skipped: false };
}

/**
 * Parallel slot: dedicated worktree, always removed in finally.
 * @param {number} i
 */
async function oneSlotParallel(i) {
	const snap = readRoadmapStatus(root);
	if (snap.counts.todo === 0) {
		return { code: 0, reason: "empty", skipped: true };
	}

	/** @type {{ path: string, branch: string, name: string } | null} */
	let wt = null;
	try {
		wt = createSwarmWorktree(root, { slot: i, waveId });
		const wtPrompt = [
			...promptArgs,
			// Reinforce isolation: agent must stay inside this worktree cwd
			`You are running inside git worktree ${wt.path} on branch ${wt.branch}. Stay in this directory. Commit on this branch only. Do not create other worktrees.`,
		];

		const { code, reason } = await runOpencodeOnce({
			label: `swarm ${i}/${wave} (parallel ${wt.name})`,
			promptArgs: wtPrompt,
			extraFlags,
			cwd: wt.path,
		});

		// Integrate commits into main before deleting the worktree
		const pending = commitsOnBranch(root, wt.branch);
		if (pending.length > 0) {
			const mergeResult = await withMergeLock(async () => {
				console.log(
					`[merge] ${wt.branch} → main (${pending.length} commit(s))`,
				);
				return mergeSwarmBranch(root, wt.branch);
			});
			if (!mergeResult.ok) {
				mergeFails++;
				console.error(
					`[merge] FAILED ${wt.branch}: ${mergeResult.reason}` +
						(mergeResult.failedAt ? ` at ${mergeResult.failedAt}` : "") +
						(mergeResult.detail ? `\n${mergeResult.detail}` : ""),
				);
				console.error(
					`[merge] commits left only on branch until cleanup deletes it: ${pending.join(" ")}`,
				);
			} else if (mergeResult.reason !== "no-commits") {
				console.log(`[merge] ok (${mergeResult.reason}) ${wt.branch}`);
			}
		}

		return { code, reason, skipped: false };
	} catch (e) {
		console.error(
			`[swarm] slot ${i} worktree error: ${/** @type {Error} */ (e).message}`,
		);
		return { code: 1, reason: "error", skipped: false };
	} finally {
		// NEVER leave a dangling worktree — success, fail, stall, or throw
		if (wt) {
			try {
				removeWorktree(root, wt.path, {
					branch: wt.branch,
					deleteBranch: true,
				});
			} catch (e) {
				console.error(
					`[worktree] finally-remove failed ${wt.path}: ${/** @type {Error} */ (e).message}`,
				);
				// global sweep will retry
			}
		}
	}
}

try {
	if (mode === "serial") {
		for (let i = 1; i <= wave; i++) {
			const r = await oneSlotSerial(i);
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
					cleanupAllSwarmWorktrees(root, { label: "swarm-stall-abort" });
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
		// Bound concurrency to wave size (already); all start together
		const results = await Promise.all(slots.map((i) => oneSlotParallel(i)));
		for (const r of results) {
			if (r.skipped) continue;
			ran++;
			if (r.reason === "stall") stalls++;
			else if (r.code !== 0) errors++;
		}
	}
} finally {
	// Absolute guarantee: no swarm worktrees remain after the wave
	cleanupAllSwarmWorktrees(root, { label: "swarm-end" });
}

const after = readRoadmapStatus(root);
const doneDelta = after.counts.done - before.counts.done;
const todoDelta = before.counts.todo - after.counts.todo;

console.log(
	`\n===== swarm done: ran=${ran}/${wave} stalls=${stalls} errors=${errors} mergeFails=${mergeFails} doneΔ=${doneDelta} todoΔ=${todoDelta} remaining_todo=${after.counts.todo} =====`,
);

if (after.counts.todo === 0) {
	console.log("roadmap complete (no todo left)");
	process.exit(stalls > 0 || errors > 0 || mergeFails > 0 ? 1 : 0);
}

if (ran > 0 && doneDelta === 0 && todoDelta === 0) {
	console.error("no roadmap progress this wave — exit 2");
	process.exit(2);
}

process.exit(stalls > 0 || errors > 0 || mergeFails > 0 ? 1 : 0);
