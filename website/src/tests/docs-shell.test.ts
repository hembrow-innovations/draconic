import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import {
  extractPageOutline,
  loadMarkdownPage,
  renderMarkdown,
} from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const shellDir = join(srcDir, "features", "docs", "DocsShell");
const outlineDir = join(srcDir, "features", "docs", "OnThisPage");

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
  expect(statSync(outlineDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "OnThisPage.tsx",
    "OnThisPage.types.ts",
    "OnThisPage.variants.ts",
  ]) {
    expect(statSync(join(outlineDir, name)).isFile()).toBe(true);
  }

  const shell = readFileSync(join(shellDir, "DocsShell.tsx"), "utf8");
  const types = readFileSync(join(shellDir, "DocsShell.types.ts"), "utf8");
  const variants = readFileSync(join(shellDir, "DocsShell.variants.ts"), "utf8");
  const outline = readFileSync(join(outlineDir, "OnThisPage.tsx"), "utf8");
  const outlineVariants = readFileSync(
    join(outlineDir, "OnThisPage.variants.ts"),
    "utf8",
  );
  const learnPage = readFileSync(
    join(srcDir, "features", "learn", "LearnPage", "LearnPage.tsx"),
    "utf8",
  );
  const referencePage = readFileSync(
    join(srcDir, "features", "reference", "ReferencePage", "ReferencePage.tsx"),
    "utf8",
  );
  const pagerVariants = readFileSync(
    join(srcDir, "features", "learn", "LearnPager", "LearnPager.variants.ts"),
    "utf8",
  );
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );
  const install = loadMarkdownPage("install");

  expect(home).not.toContain("DocsShell");
  expect(home).toContain("HomeHero");
  expect(root).not.toContain("DocsShell");
  expect(root).not.toContain("features/docs");

  expect(header).toContain("<aside");
  expect(header).toContain("<nav");
  expect(header).toContain("LearnNav");
  expect(header).toContain("ReferenceNav");
  expect(header).toContain('"aria-current": "page"');
  expect(shell).not.toContain("<aside");
  expect(shell).toContain("<article");
  expect(shell).toContain("Badge");
  expect(shell).toContain('from "../../../components/Badge"');
  expect(shell).toContain("renderMarkdown");
  expect(shell).toContain("status");
  expect(shell).toContain("shipped");
  expect(shell).toContain("not-yet");
  expect(shell).toContain("public-site.chrome:docs-sidebar");
  expect(shell).toContain("public-site.chrome:docs-article-order");
  expect(shell).toContain("public-site.chrome:on-page-toc");
  expect(shell).toContain("OnThisPage");
  expect(shell).toContain("<OnThisPage body={body}");

  const article = shell.slice(shell.indexOf("<article"));
  const kickerAt = article.indexOf("docsShellKickerVariants");
  const headingAt = article.indexOf("<h1");
  const badgeAt = article.indexOf("<Badge");
  const outlineAt = article.indexOf("OnThisPage");
  const footerAt = article.indexOf("<footer");
  expect(kickerAt).toBeGreaterThan(-1);
  expect(headingAt).toBeGreaterThan(kickerAt);
  expect(badgeAt).toBeGreaterThan(headingAt);
  expect(outlineAt).toBeGreaterThan(badgeAt);
  expect(footerAt).toBeGreaterThan(outlineAt);
  expect(article.slice(outlineAt, footerAt)).toContain("dangerouslySetInnerHTML");

  expect(outline).toContain("public-site.chrome:on-page-toc");
  expect(outline).toContain("extractPageOutline");
  expect(outline).toContain("items.length === 0");
  expect(outline).toContain('aria-label="On this page"');
  expect(outline).toContain("On this page");
  expect(outline).toContain("href={`#${item.id}`}");
  expect(outline).toContain("useLocation");
  expect(outline).toContain("location.hash");
  expect(outline).toContain("item.id");
  expect(outline).toContain('"aria-current": "true"');
  expect(outline).not.toContain('"aria-current": "page"');
  expect(outline).not.toMatch(/<a[^>]*aria-current/);
  expect(outline).not.toMatch(/playground/i);
  expect(outlineVariants).toContain('from "class-variance-authority"');
  expect(outlineVariants).toContain("cva(");
  expect(outlineVariants).toContain("text-muted");
  expect(outlineVariants).toContain("aria-[current=true]:text-ink");
  expect(outlineVariants).not.toMatch(/aria-\[current=true\]:text-muted/);
  expect(outlineVariants).toContain("focus-visible:ring-accent");
  expect(outlineVariants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(outlineVariants).not.toMatch(/\bmax-w-sm\b/);
  expect(shell).not.toContain("HomeHero");
  expect(shell).not.toContain("JavaScript you already know");
  expect(shell).not.toMatch(/playground/i);
  expect(shell).not.toContain("docs/specs");
  expect(shell).not.toContain("from JavaScript");
  expect(shell).not.toContain("from systems");
  expect(shell).not.toContain("Dual worlds");

  expect(types).toContain("status");
  expect(types).toContain("badgeVariants");
  expect(types).toContain("kicker");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).toContain("docsShellKickerVariants");
  expect(variants).toContain("kicker");
  expect(variants).toContain("uppercase");
  expect(variants).toContain("tracking-widest");
  expect(variants).toContain("docsShellFooterVariants");
  expect(variants).toContain("bg-code");
  expect(variants).toContain("[&_pre]");
  expect(variants).toContain("[&_pre]:overflow-x-auto");
  expect(variants).not.toContain("[&_pre]:overflow-visible");
  expect(variants).not.toContain("[&_pre]:overflow-x-visible");
  expect(variants).toContain("[&_code]");
  expect(variants).toContain("[&_h2]:mt-8");
  expect(variants).toContain("[&_h2]:border-t");
  expect(variants).toContain("[&_h2]:border-line");
  expect(variants).toContain("[&_h2]:pt-8");
  expect(variants).toContain("[&_h2>a]:text-ink");
  expect(variants).toContain("[&_h2>a]:no-underline");
  expect(variants).toContain("[&_h2>a]:focus-visible:ring-2");
  expect(variants).toContain("[&_h2>a]:focus-visible:ring-accent");
  expect(variants).not.toContain("[&_h2:first-of-type]");
  expect(variants).toContain("[&>div>p:first-child]");
  expect(variants).toContain("border-t");
  expect(variants).toContain("border-line");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(shell).toContain("docsShellVariants");
  expect(shell).toContain("docsShellKickerVariants");
  expect(shell).toContain("docsShellFooterVariants");
  expect(shell).toContain("<footer");
  expect(shell).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(shell).not.toMatch(/\bmax-w-sm\b/);
  expect(shell).not.toMatch(/bg-blue-500/);

  expect(learnPage).toContain("kicker={page.section}");
  expect(learnPage).toContain("LearnPager");
  expect(referencePage).toContain("kicker={page.section}");
  expect(referencePage).toContain("ReferencePager");
  expect(referencePage).toContain("page.slug");
  expect(referencePage).not.toContain("LearnPager");

  expect(pagerVariants).toContain("text-link");
  expect(pagerVariants).toContain("border-t");
  expect(pagerVariants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);

  expect(install.status).toBe("shipped");
  expect(renderMarkdown(install.body)).toContain("<h1>Install</h1>");

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});

test("page outline", () => {
  const install = loadMarkdownPage("install");
  const html = renderMarkdown(install.body);
  const outline = extractPageOutline(install.body);
  expect(outline.map((item) => item.id)).toEqual([
    "from-source",
    "zed-editor",
    "reproducibility",
  ]);
  expect(outline.map((item) => item.text)).toEqual([
    "From source",
    "Zed editor",
    "Reproducibility",
  ]);
  for (const item of outline) {
    expect(html).toContain(
      `<h2 id="${item.id}"><a href="#${item.id}">${item.text}</a></h2>`,
    );
  }

  const outlineSource = readFileSync(join(outlineDir, "OnThisPage.tsx"), "utf8");
  const outlineVariants = readFileSync(
    join(outlineDir, "OnThisPage.variants.ts"),
    "utf8",
  );
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );
  expect(outline.map((item) => item.id)).toContain("zed-editor");
  expect(outlineSource).toContain("location.hash");
  expect(outlineSource).toContain("item.id");
  expect(outlineSource).toContain('"aria-current": "true"');
  expect(outlineSource).not.toContain('"aria-current": "page"');
  expect(outlineSource).not.toMatch(/<a[^>]*aria-current/);
  expect(outlineVariants).toContain("text-muted");
  expect(outlineVariants).toContain("aria-[current=true]:text-ink");
  expect(outlineVariants).not.toMatch(/aria-\[current=true\]:text-muted/);
  expect(header).toContain('"aria-current": "page"');

  expect(extractPageOutline(loadMarkdownPage("dual-worlds").body)).toEqual([]);

  const cli = extractPageOutline(loadMarkdownPage("cli").body);
  expect(cli.map((item) => item.text)).toEqual([
    "parse",
    "extract",
    "check",
    "fmt",
    "doc",
    "build",
    "run",
    "repl",
    "test",
    "version",
    "help",
    "bindgen",
    "Permissions",
    "Shebang",
  ]);
  expect(cli.map((item) => item.id)).toContain("shebang");
  expect(cli.every((item) => item.level === 2)).toBe(true);
});
