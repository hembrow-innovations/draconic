import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import {
  loadMarkdownPage,
  renderMarkdown,
  splitMarkdownHtml,
} from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const fenceDir = join(srcDir, "components", "CodeFence");

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

test("code fence copy", () => {
  expect(statSync(fenceDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "CodeFence.tsx",
    "CodeFence.types.ts",
    "CodeFence.variants.ts",
  ]) {
    expect(statSync(join(fenceDir, name)).isFile()).toBe(true);
  }

  const fence = readFileSync(join(fenceDir, "CodeFence.tsx"), "utf8");
  const types = readFileSync(join(fenceDir, "CodeFence.types.ts"), "utf8");
  const variants = readFileSync(
    join(fenceDir, "CodeFence.variants.ts"),
    "utf8",
  );
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );
  const lib = readFileSync(join(srcDir, "lib", "content", "index.ts"), "utf8");

  expect(fence).toContain("public-site.fences:copy");
  expect(fence).toContain('type="button"');
  expect(fence).toContain("navigator.clipboard.writeText");
  expect(fence).toContain("Copy");
  expect(fence).toContain("Copied");
  expect(fence).toContain("aria-label");
  expect(fence).toContain("<pre");
  expect(fence).toContain("<code");
  expect(fence).toContain("codeFenceVariants");
  expect(fence).not.toMatch(/playground/i);
  expect(fence).not.toContain("docs/");
  expect(fence).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(fence).not.toMatch(/\bmax-w-sm\b/);
  expect(fence).not.toMatch(/\babsolute\b/);

  expect(types).toContain("code: string");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-mono");
  expect(variants).toContain("min-h-11");
  expect(variants).toContain("focus-visible:ring-2");
  expect(variants).toContain("focus-visible:ring-accent");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);
  expect(variants).not.toMatch(/\babsolute\b/);

  expect(shell).toContain("CodeFence");
  expect(shell).toContain("splitMarkdownHtml");
  expect(shell).toContain("public-site.fences:copy");
  expect(shell).not.toMatch(/playground/i);

  expect(lib).toContain("splitMarkdownHtml");

  const installHtml = renderMarkdown(loadMarkdownPage("install").body);
  const blocks = splitMarkdownHtml(installHtml);
  const fences = blocks.filter((block) => block.kind === "fence");
  expect(fences.length).toBeGreaterThan(0);
  expect(
    fences.some((block) => block.code.includes("curl -fsSL")),
  ).toBe(true);
  expect(
    fences.some((block) =>
      block.code.includes("let console = globalThis.console;"),
    ),
  ).toBe(true);
  expect(blocks.some((block) => block.kind === "html")).toBe(true);

  const mixed = splitMarkdownHtml(
    "<p>before</p>\n<pre><code>let n = 1;\n</code></pre>\n<p>after</p>\n",
  );
  expect(mixed).toEqual([
    { kind: "html", html: "<p>before</p>\n" },
    { kind: "fence", code: "let n = 1;\n" },
    { kind: "html", html: "<p>after</p>\n" },
  ]);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
