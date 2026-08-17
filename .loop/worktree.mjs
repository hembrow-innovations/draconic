#!/usr/bin/env node
// Git worktree lifecycle for swarm workers.
// Every create is paired with remove; cleanupAllSwarmWorktrees is idempotent
// and safe to run on start/exit/signal so nothing dangles under .loop/worktrees/.
import { execFileSync, spawnSync } from "node:child_process";
import {
	existsSync,
	mkdirSync,
	readdirSync,
	rmSync,
	statSync,
} from "node:fs";
import { join, resolve } from "node:path";

export const SWARM_WT_DIR = ".loop/worktrees";
export const SWARM_BRANCH_PREFIX = "swarm/";

function git(repoRoot, args, opts = {}) {
	return execFileSync("git", args, {
		cwd: repoRoot,
		encoding: "utf8",
		stdio: opts.stdio ?? ["ignore", "pipe", "pipe"],
		...opts,
	});
}

function gitOk(repoRoot, args) {
	const r = spawnSync("git", args, {
		cwd: repoRoot,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
	return {
		ok: r.status === 0,
		status: r.status ?? 1,
		stdout: r.stdout || "",
		stderr: r.stderr || "",
	};
}

export function repoRoot(cwd = process.cwd()) {
	const out = git(cwd, ["rev-parse", "--show-toplevel"]).trim();
	return out;
}

export function worktreeRoot(root) {
	return resolve(root, SWARM_WT_DIR);
}

/** @returns {{ path: string, branch: string | null, bare: boolean, detached: boolean }[]} */
export function listWorktrees(root) {
	const out = git(root, ["worktree", "list", "--porcelain"]);
	/** @type {{ path: string, branch: string | null, bare: boolean, detached: boolean }[]} */
	const items = [];
	/** @type {{ path?: string, branch: string | null, bare: boolean, detached: boolean }} */
	let cur = { branch: null, bare: false, detached: false };
	for (const line of out.split("\n")) {
		if (line.startsWith("worktree ")) {
			if (cur.path) items.push(/** @type {any} */ (cur));
			cur = {
				path: line.slice("worktree ".length),
				branch: null,
				bare: false,
				detached: false,
			};
		} else if (line.startsWith("branch ")) {
			const ref = line.slice("branch ".length);
			cur.branch = ref.startsWith("refs/heads/")
				? ref.slice("refs/heads/".length)
				: ref;
		} else if (line === "bare") {
			cur.bare = true;
		} else if (line === "detached") {
			cur.detached = true;
		} else if (line === "") {
			if (cur.path) items.push(/** @type {any} */ (cur));
			cur = { branch: null, bare: false, detached: false };
		}
	}
	if (cur.path) items.push(/** @type {any} */ (cur));
	return items;
}

export function isSwarmWorktreePath(root, path) {
	const base = worktreeRoot(root);
	const resolved = resolve(path);
	return resolved === base || resolved.startsWith(base + "/");
}

export function isSwarmBranch(branch) {
	return typeof branch === "string" && branch.startsWith(SWARM_BRANCH_PREFIX);
}

/**
 * Remove one worktree path. Always best-effort: force remove, rm -rf, prune.
 * Optionally delete its swarm/* branch.
 */
export function removeWorktree(root, path, { branch = null, deleteBranch = true } = {}) {
	const resolved = resolve(path);
	console.error(`[worktree] remove ${resolved}`);

	// Prefer git worktree remove --force
	let removed = gitOk(root, ["worktree", "remove", "--force", resolved]);
	if (!removed.ok) {
		// Path may already be gone from git's view but dir remains, or locked
		if (existsSync(resolved)) {
			try {
				rmSync(resolved, { recursive: true, force: true });
			} catch (e) {
				console.error(
					`[worktree] rmSync failed ${resolved}: ${/** @type {Error} */ (e).message}`,
				);
			}
		}
		gitOk(root, ["worktree", "prune"]);
		// retry remove after prune
		removed = gitOk(root, ["worktree", "remove", "--force", resolved]);
		if (!removed.ok && existsSync(resolved)) {
			try {
				rmSync(resolved, { recursive: true, force: true });
			} catch {
				/* last resort already tried */
			}
		}
	}

	gitOk(root, ["worktree", "prune"]);

	if (deleteBranch && branch && isSwarmBranch(branch)) {
		const del = gitOk(root, ["branch", "-D", branch]);
		if (del.ok) {
			console.error(`[worktree] deleted branch ${branch}`);
		}
	}

	return !existsSync(resolved);
}

/**
 * Create a disposable swarm worktree + branch from HEAD.
 * @returns {{ path: string, branch: string, name: string }}
 */
export function createSwarmWorktree(root, { slot, waveId = "0" } = {}) {
	const rootResolved = resolve(root);
	const base = worktreeRoot(rootResolved);
	mkdirSync(base, { recursive: true });

	const stamp = `${Date.now().toString(36)}-${process.pid.toString(36)}`;
	const name = `w${waveId}-s${slot}-${stamp}`;
	const branch = `${SWARM_BRANCH_PREFIX}${name}`;
	const path = join(base, name);

	if (existsSync(path)) {
		removeWorktree(rootResolved, path, { branch, deleteBranch: true });
	}

	const add = gitOk(rootResolved, ["worktree", "add", "-b", branch, path, "HEAD"]);
	if (!add.ok) {
		throw new Error(
			`git worktree add failed for ${path}: ${add.stderr || add.stdout}`,
		);
	}
	console.error(`[worktree] created ${path} (branch ${branch})`);
	return { path, branch, name };
}

/**
 * Commits on branch not in main HEAD (exclusive).
 */
export function commitsOnBranch(root, branch) {
	const r = gitOk(root, ["rev-list", "--reverse", `HEAD..${branch}`]);
	if (!r.ok) return [];
	return r.stdout
		.trim()
		.split("\n")
		.map((s) => s.trim())
		.filter(Boolean);
}

/**
 * Merge worker branch into current branch of root. Returns { ok, reason }.
 */
export function mergeSwarmBranch(root, branch) {
	const commits = commitsOnBranch(root, branch);
	if (commits.length === 0) {
		return { ok: true, reason: "no-commits", commits: [] };
	}

	// Ensure clean index before merge
	const dirty = gitOk(root, ["status", "--porcelain"]);
	if (dirty.ok && dirty.stdout.trim()) {
		return {
			ok: false,
			reason: "main-dirty",
			commits,
			detail: dirty.stdout.trim().slice(0, 500),
		};
	}

	const merge = gitOk(root, [
		"merge",
		"--no-ff",
		"--no-edit",
		"-m",
		`swarm: merge ${branch}`,
		branch,
	]);
	if (merge.ok) {
		return { ok: true, reason: "merged", commits };
	}

	// Abort failed merge so main stays usable
	gitOk(root, ["merge", "--abort"]);

	// Try cherry-pick each commit in order
	for (const sha of commits) {
		const cp = gitOk(root, ["cherry-pick", "--ff", sha]);
		if (!cp.ok) {
			gitOk(root, ["cherry-pick", "--abort"]);
			return {
				ok: false,
				reason: "conflict",
				commits,
				failedAt: sha,
				detail: (cp.stderr || cp.stdout).slice(0, 800),
			};
		}
	}
	return { ok: true, reason: "cherry-picked", commits };
}

/**
 * Nuke every swarm worktree under .loop/worktrees and matching branches.
 * Also drops git worktree entries whose path is under that dir.
 * Safe / idempotent.
 */
export function cleanupAllSwarmWorktrees(root, { label = "cleanup" } = {}) {
	const rootResolved = resolve(root);
	const base = worktreeRoot(rootResolved);
	console.error(`[worktree] ${label}: sweeping swarm worktrees under ${base}`);

	// 1) Registered worktrees whose path is under .loop/worktrees
	const registered = listWorktrees(rootResolved);
	for (const wt of registered) {
		if (!isSwarmWorktreePath(rootResolved, wt.path)) continue;
		// never touch the main worktree
		if (resolve(wt.path) === rootResolved) continue;
		removeWorktree(rootResolved, wt.path, {
			branch: wt.branch,
			deleteBranch: isSwarmBranch(wt.branch),
		});
	}

	// 2) Orphan directories left on disk
	if (existsSync(base)) {
		let entries = [];
		try {
			entries = readdirSync(base);
		} catch {
			entries = [];
		}
		for (const name of entries) {
			const p = join(base, name);
			try {
				if (!statSync(p).isDirectory()) {
					rmSync(p, { force: true });
					continue;
				}
			} catch {
				continue;
			}
			removeWorktree(rootResolved, p, {
				branch: `${SWARM_BRANCH_PREFIX}${name}`,
				deleteBranch: true,
			});
		}
		// remove empty root
		try {
			const left = readdirSync(base);
			if (left.length === 0) rmSync(base, { recursive: true, force: true });
		} catch {
			/* ignore */
		}
	}

	// 3) Prune + delete any leftover swarm/* branches with no worktree
	gitOk(rootResolved, ["worktree", "prune"]);
	const branches = gitOk(rootResolved, [
		"for-each-ref",
		"--format=%(refname:short)",
		`refs/heads/${SWARM_BRANCH_PREFIX}`,
	]);
	if (branches.ok && branches.stdout.trim()) {
		const still = new Set(
			listWorktrees(rootResolved)
				.map((w) => w.branch)
				.filter(Boolean),
		);
		for (const b of branches.stdout.trim().split("\n")) {
			const branch = b.trim();
			if (!branch || still.has(branch)) continue;
			const del = gitOk(rootResolved, ["branch", "-D", branch]);
			if (del.ok) console.error(`[worktree] deleted orphan branch ${branch}`);
		}
	}

	const remaining = listWorktrees(rootResolved).filter((w) =>
		isSwarmWorktreePath(rootResolved, w.path),
	);
	if (remaining.length > 0) {
		console.error(
			`[worktree] WARNING: ${remaining.length} swarm worktree(s) still registered:`,
		);
		for (const w of remaining) console.error(`  - ${w.path}`);
		return false;
	}
	console.error(`[worktree] ${label}: clean`);
	return true;
}

/** Install process handlers that always sweep swarm worktrees. */
export function installWorktreeCleanupHandlers(root) {
	const rootResolved = resolve(root);
	let ran = false;
	const run = (why) => {
		if (ran) return;
		ran = true;
		try {
			cleanupAllSwarmWorktrees(rootResolved, { label: `on-${why}` });
		} catch (e) {
			console.error(
				`[worktree] cleanup on ${why} failed: ${/** @type {Error} */ (e).message}`,
			);
		}
	};

	for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
		process.on(sig, () => {
			run(sig);
			process.exit(130);
		});
	}
	process.on("exit", () => run("exit"));
	process.on("uncaughtException", (err) => {
		console.error("[worktree] uncaughtException", err);
		run("uncaughtException");
		process.exit(1);
	});
	process.on("unhandledRejection", (err) => {
		console.error("[worktree] unhandledRejection", err);
		run("unhandledRejection");
		process.exit(1);
	});
}

// CLI: node .loop/worktree.mjs cleanup
const isCli =
	process.argv[1] &&
	resolve(process.argv[1]).endsWith(`${join("loop", "worktree.mjs")}`);
if (isCli || process.argv[1]?.endsWith("worktree.mjs")) {
	const cmd = process.argv[2] || "cleanup";
	const root = repoRoot();
	if (cmd === "cleanup" || cmd === "prune") {
		const ok = cleanupAllSwarmWorktrees(root, { label: "cli" });
		process.exit(ok ? 0 : 1);
	} else if (cmd === "list") {
		for (const w of listWorktrees(root)) {
			const mark = isSwarmWorktreePath(root, w.path) ? " [swarm]" : "";
			console.log(`${w.path}  ${w.branch || "(detached)"}${mark}`);
		}
	} else {
		console.error("Usage: node .loop/worktree.mjs cleanup|list");
		process.exit(1);
	}
}
