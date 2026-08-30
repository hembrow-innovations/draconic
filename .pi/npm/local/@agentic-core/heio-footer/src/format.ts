// Edit formatFooterLine to change the one-line footer.
import { existsSync } from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";

export type FooterFields = {
	cwd: string;
	teamStatus?: string;
	tokens: number | null;
	contextWindow: number;
	cost: number;
	autoCompact?: boolean;
	model: string;
	effort?: string;
};

function findGitRoot(cwd: string): string | undefined {
	let dir = resolve(cwd);
	while (true) {
		if (existsSync(join(dir, ".git"))) return dir;
		const parent = dirname(dir);
		if (parent === dir) return undefined;
		dir = parent;
	}
}

function posixJoin(parts: string[]): string {
	return parts.filter((part) => part.length > 0).join("/");
}

export function formatCwdFromRoot(cwd: string): string {
	const abs = resolve(cwd);
	const root = findGitRoot(abs);
	if (!root) return basename(abs);
	const rel = relative(root, abs);
	if (!rel || rel === ".") return basename(root);
	return posixJoin([basename(root), ...rel.split(sep)]);
}

export function formatTokens(count: number): string {
	if (count < 1000) return count.toString();
	if (count < 10000) return `${(count / 1000).toFixed(1)}k`;
	if (count < 1000000) return `${Math.round(count / 1000)}k`;
	if (count < 10000000) return `${(count / 1000000).toFixed(1)}M`;
	return `${Math.round(count / 1000000)}M`;
}

function isWideCodePoint(code: number): boolean {
	if (code >= 0x1100 && code <= 0x115f) return true;
	if (code === 0x2329 || code === 0x232a) return true;
	if (code >= 0x2e80 && code <= 0xa4cf) return true;
	if (code >= 0xac00 && code <= 0xd7a3) return true;
	if (code >= 0xf900 && code <= 0xfaff) return true;
	if (code >= 0xfe10 && code <= 0xfe19) return true;
	if (code >= 0xfe30 && code <= 0xfe6f) return true;
	if (code >= 0xff00 && code <= 0xff60) return true;
	if (code >= 0xffe0 && code <= 0xffe6) return true;
	if (code >= 0x1f300 && code <= 0x1f64f) return true;
	if (code >= 0x1f900 && code <= 0x1f9ff) return true;
	if (code >= 0x20000 && code <= 0x3fffd) return true;
	return false;
}

function charVisibleWidth(char: string): number {
	const code = char.codePointAt(0);
	if (code === undefined) return 0;
	if (code <= 31 || code === 127) return 0;
	if (code >= 0x300 && code <= 0x36f) return 0;
	if (isWideCodePoint(code)) return 2;
	return 1;
}

export function visibleWidth(text: string): number {
	let width = 0;
	for (const char of text) {
		width += charVisibleWidth(char);
	}
	return width;
}

export function clipToVisibleWidth(text: string, width: number): string {
	if (width <= 0) return "";
	if (visibleWidth(text) <= width) return text;
	let out = "";
	let used = 0;
	for (const char of text) {
		const next = charVisibleWidth(char);
		if (used + next > width) break;
		out += char;
		used += next;
	}
	return out;
}

export function formatFooterLine(fields: FooterFields): string {
	const tokens = fields.tokens === null ? "?" : formatTokens(fields.tokens);
	const parts = [
		fields.cwd,
		fields.teamStatus?.trim() || undefined,
		`${tokens}/${formatTokens(fields.contextWindow)}`,
		`$${fields.cost.toFixed(3)}`,
		fields.autoCompact ? "(auto)" : undefined,
		fields.model,
		fields.effort,
	].filter((part): part is string => Boolean(part && part.length > 0));
	return parts.join(" ");
}
