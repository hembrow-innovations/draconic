#!/usr/bin/env node
// Reinstall the heio-stack profile from a local agentic-core checkout.
// Usage: node scripts/install-heio.mjs
//        AGENTIC_CORE=/path/to/agentic-core node scripts/install-heio.mjs
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const dest = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const src = resolve(
	process.env.AGENTIC_CORE || resolve(dest, "../agentic-core"),
);
if (!existsSync(resolve(src, "profiles")) || !existsSync(resolve(src, "ai/skills"))) {
	console.error(`agentic-core not found at ${src}. Set AGENTIC_CORE.`);
	process.exit(1);
}

const result = spawnSync(
	"pnpm",
	["exec", "agentic-core", "install", dest, "--profile", "heio-stack"],
	{ cwd: src, stdio: "inherit" },
);
process.exit(result.status ?? 1);
