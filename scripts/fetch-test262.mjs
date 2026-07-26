#!/usr/bin/env node
// Fetch tc39/test262 at a pinned commit into third_party/test262 (gitignored).
// Idempotent: skips clone when the working tree already matches PINNED_SHA.
//
//   node scripts/fetch-test262.mjs
//   TEST262_ROOT=/path node scripts/fetch-test262.mjs   # still uses default dest
//
// Does not commit the suite. CI and local agents opt in by running this script.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** Pinned tc39/test262 commit. Bump deliberately when expanding the allowlist. */
export const PINNED_SHA = "07dbcbca04c5ac73eefd752eb0a67a893c159374";

export const REPO_URL = "https://github.com/tc39/test262.git";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const DEFAULT_DEST = path.join(ROOT, "third_party", "test262");

function run(cmd, args, opts = {}) {
	const r = spawnSync(cmd, args, {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
		...opts,
	});
	if (r.error) throw r.error;
	if (r.status !== 0) {
		const err = (r.stderr || r.stdout || "").trim();
		throw new Error(`${cmd} ${args.join(" ")} failed (exit ${r.status}): ${err}`);
	}
	return (r.stdout || "").trim();
}

function git(dest, args) {
	return run("git", ["-C", dest, ...args]);
}

/**
 * Ensure `dest` is a test262 checkout at PINNED_SHA.
 * @param {string} [dest]
 * @returns {{ dest: string, sha: string, action: "clone" | "fetch" | "skip" }}
 */
export function fetchTest262(dest = DEFAULT_DEST) {
	fs.mkdirSync(path.dirname(dest), { recursive: true });

	const gitDir = path.join(dest, ".git");
	if (!fs.existsSync(gitDir)) {
		if (fs.existsSync(dest)) {
			// Partial/non-git directory — replace cleanly.
			fs.rmSync(dest, { recursive: true, force: true });
		}
		console.error(`cloning ${REPO_URL} → ${dest}`);
		run("git", [
			"clone",
			"--filter=blob:none",
			"--no-checkout",
			REPO_URL,
			dest,
		]);
		git(dest, ["fetch", "--depth", "1", "origin", PINNED_SHA]);
		git(dest, ["checkout", PINNED_SHA]);
		return { dest, sha: PINNED_SHA, action: "clone" };
	}

	let head = "";
	try {
		head = git(dest, ["rev-parse", "HEAD"]);
	} catch {
		head = "";
	}

	if (head === PINNED_SHA) {
		console.error(`test262 already at ${PINNED_SHA} (${dest})`);
		return { dest, sha: PINNED_SHA, action: "skip" };
	}

	console.error(`updating test262 ${head || "?"} → ${PINNED_SHA}`);
	git(dest, ["fetch", "--depth", "1", "origin", PINNED_SHA]);
	git(dest, ["checkout", PINNED_SHA]);
	return { dest, sha: PINNED_SHA, action: "fetch" };
}

function main() {
	const result = fetchTest262();
	console.log(JSON.stringify(result));
}

const isMain =
	process.argv[1] &&
	path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
	try {
		main();
	} catch (e) {
		console.error(e instanceof Error ? e.message : e);
		process.exit(1);
	}
}
