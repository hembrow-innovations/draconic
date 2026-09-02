#!/usr/bin/env node
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(root, ".pi", "frameworks", "hivemind", "src", "cli.ts");

if (!existsSync(cli)) {
	console.error("missing .pi/frameworks/hivemind/src/cli.ts");
	console.error("reinstall: node scripts/install-heio.mjs");
	process.exit(1);
}

const child = spawn(
	process.execPath,
	["--experimental-strip-types", cli, ...process.argv.slice(2)],
	{ cwd: root, stdio: "inherit" },
);
child.on("exit", (code) => {
	process.exit(code ?? 1);
});
child.on("error", (err) => {
	console.error(err instanceof Error ? err.message : String(err));
	process.exit(1);
});
