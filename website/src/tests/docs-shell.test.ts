import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const shellDir = join(srcDir, "features", "docs", "DocsShell");

function walkProductFiles(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name === "tests") {
        continue;
      }
      out.push(...walkProductFiles(path));
      continue;
    }
    if (name.endsWith(".gen.ts") || name === "routeTree.gen.ts") {
      continue;
    }
    if (name.endsWith(".tsx") || name.endsWith(".ts")) {
      out.push(path);
    }
  }
  return out;
}

test("docs shell", () => {
  expect(statSync(shellDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "DocsShell.tsx",
    "DocsShell.types.ts",
    "DocsShell.variants.ts",
  ]) {
    expect(statSync(join(shellDir, name)).isFile()).toBe(true);
  }

  const shell = readFileSync(join(shellDir, "DocsShell.tsx"), "utf8");
  const types = readFileSync(join(shellDir, "DocsShell.types.ts"), "utf8");
  const variants = readFileSync(join(shellDir, "DocsShell.variants.ts"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const install = loadMarkdownPage("install");

  expect(home).not.toContain("DocsShell");
  expect(home).toContain("HomeHero");
  expect(root).not.toContain("DocsShell");
  expect(root).not.toContain("features/docs");

  expect(shell).toContain("<aside");
  expect(shell).toContain("<nav");
  expect(shell).toContain("<article");
  expect(shell).toContain("Badge");
  expect(shell).toContain('from "../../../components/Badge"');
  expect(shell).toContain("renderMarkdown");
  expect(shell).toContain("status");
  expect(shell).toContain("shipped");
  expect(shell).toContain("not-yet");
  expect(shell).not.toContain("HomeHero");
  expect(shell).not.toContain("JavaScript you already know");
  expect(shell).not.toMatch(/playground/i);
  expect(shell).not.toContain("docs/specs");
  expect(shell).not.toContain("from JavaScript");
  expect(shell).not.toContain("from systems");
  expect(shell).not.toContain("Dual worlds");

  expect(types).toContain("status");
  expect(types).toContain("badgeVariants");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(shell).toContain("docsShellVariants");
  expect(shell).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(shell).not.toMatch(/\bmax-w-sm\b/);
  expect(shell).not.toMatch(/bg-blue-500/);

  expect(install.status).toBe("shipped");
  expect(renderMarkdown(install.body)).toContain("<h1>Install</h1>");

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
