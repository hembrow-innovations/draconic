#!/usr/bin/env node
// Outer orchestrator: loop swarm waves until ROADMAP has no todo rows.
// Does not hold LLM context — each unit of work is a child opencode process
// inside swarm. This process only counts roadmap rows and spawns swarms.
//
// Usage:
//   node .loop/opencode-orchestrate.mjs
//   node .loop/opencode-orchestrate.mjs parallel wave=10
//   node .loop/opencode-orchestrate.mjs wave=5 -- -m xai/grok-4.5
//
// Env:
//   WAVE=10              default wave size
//   SWARM_MODE=serial    serial|parallel
//   MAX_WAVES=0          0 = unlimited
//   MAX_NO_PROGRESS=3    stop after this many consecutive no-progress waves
//   SLEEP=0              seconds between waves
//   STALL_SEC / STALL_ACTION  passed through to children

import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { readRoadmapStatus } from "./roadmap-status.mjs";
import {
	cleanupAllSwarmWorktrees,
	installWorktreeCleanupHandlers,
	repoRoot,
} from "./worktree.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const swarmScript = join(here, "opencode-swarm.mjs");
const root = repoRoot();
installWorktreeCleanupHandlers(root);
cleanupAllSwarmWorktrees(root, { label: "orchestrate-start" });

function parseArgs(argv) {
	/** @type {string[]} */
	const forward = [];
	let maxWaves = Number.parseInt(process.env.MAX_WAVES || "0", 10);
	let maxNoProgress = Number.parseInt(process.env.MAX_NO_PROGRESS || "3", 10);

	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		if (a === "--max-waves") {
			maxWaves = Number.parseInt(argv[++i], 10);
			continue;
		}
		const mw = /^max.waves=(\d+)$/i.exec(a);
		if (mw) {
			maxWaves = Number.parseInt(mw[1], 10);
			continue;
		}
		if (a === "--max-no-progress") {
			maxNoProgress = Number.parseInt(argv[++i], 10);
			continue;
		}
		forward.push(a);
	}
	return { forward, maxWaves, maxNoProgress };
}

const { forward, maxWaves, maxNoProgress } = parseArgs(process.argv.slice(2));
const sleepMs = (Number.parseFloat(process.env.SLEEP_WAVE || process.env.SLEEP) || 0) * 1000;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const runSwarm = () =>
	new Promise((resolve) => {
		const child = spawn(process.execPath, [swarmScript, ...forward], {
			stdio: "inherit",
			env: process.env,
		});
		child.on("close", (code) => resolve(code ?? 0));
		child.on("error", (err) => {
			console.error(`[orchestrate] swarm spawn failed: ${err.message}`);
			resolve(1);
		});
	});

console.log("orchestrate: loop swarm until ROADMAP todo=0");
console.log(`swarm args: ${forward.join(" ") || "(defaults)"}`);
console.log(
	`max_waves=${maxWaves || "∞"} max_no_progress=${maxNoProgress} sleep_between_waves=${sleepMs / 1000}s`,
);

let wave = 0;
let noProgress = 0;
let totalErrors = 0;

for (;;) {
	const st = readRoadmapStatus();
	console.log(
		`\n##### orchestrate check: todo=${st.counts.todo} in_progress=${st.counts.in_progress} done=${st.counts.done} blocked=${st.counts.blocked}`,
	);
	if (st.counts.todo === 0) {
		console.log("##### orchestrate complete: no todo items left");
		cleanupAllSwarmWorktrees(root, { label: "orchestrate-complete" });
		process.exit(totalErrors > 0 ? 1 : 0);
	}

	if (maxWaves > 0 && wave >= maxWaves) {
		console.error(`##### hit MAX_WAVES=${maxWaves} with todo=${st.counts.todo} remaining`);
		process.exit(1);
	}

	wave++;
	const beforeDone = st.counts.done;
	const beforeTodo = st.counts.todo;
	console.log(`##### wave ${wave} starting (todo=${beforeTodo})`);

	const code = await runSwarm();
	// Swarm should have cleaned its trees; sweep again in case of hard kill
	cleanupAllSwarmWorktrees(root, { label: `orchestrate-after-wave-${wave}` });
	if (code !== 0 && code !== 2) totalErrors++;

	const after = readRoadmapStatus();
	const progressed =
		after.counts.done > beforeDone || after.counts.todo < beforeTodo;

	if (after.counts.todo === 0) {
		console.log("##### orchestrate complete after wave", wave);
		cleanupAllSwarmWorktrees(root, { label: "orchestrate-complete" });
		process.exit(totalErrors > 0 || code === 1 ? 1 : 0);
	}

	if (!progressed || code === 2) {
		noProgress++;
		console.error(
			`##### no progress wave ${wave} (${noProgress}/${maxNoProgress})`,
		);
		if (noProgress >= maxNoProgress) {
			console.error(
				"##### abort: too many consecutive no-progress waves (stuck items or agent failures)",
			);
			cleanupAllSwarmWorktrees(root, { label: "orchestrate-stuck" });
			process.exit(2);
		}
	} else {
		noProgress = 0;
	}

	if (sleepMs) {
		console.log(`##### sleeping ${sleepMs / 1000}s before next wave...`);
		await sleep(sleepMs);
	}
}
