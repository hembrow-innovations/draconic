import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const routesDir = join(srcDir, "routes");
const pageDir = join(srcDir, "features", "learn", "LearnPage");

/** Chapter routes for `public-site.ia:learn-walkable`. Hub is already `/learn`. */
const learnChapters = [
  {
    slug: "install",
    title: "Install",
    status: "shipped",
    copy: "Get the toolchain, then parse, build, and run a Program before reading further.",
  },
  {
    slug: "from-javascript",
    title: "from JavaScript",
    status: "shipped",
    copy: "This landing assumes you already write JavaScript or TypeScript.",
  },
  {
    slug: "from-systems",
    title: "from systems",
    status: "not-yet",
    copy: "This landing assumes you already write Rust, Go, or C.",
  },
  {
    slug: "dual-worlds",
    title: "Dual worlds",
    status: "not-yet",
    copy: "Dual worlds is the coexistence of JS values and native types in one Program",
  },
  {
    slug: "modules",
    title: "modules",
    status: "not-yet",
    copy: "A Program may be one file or an ECMAScript Module graph.",
  },
  {
    slug: "native-types",
    title: "native types",
    status: "not-yet",
    copy: "A native type is a static, unboxed systems type.",
  },
  {
    slug: "host-io",
    title: "host I/O",
    status: "not-yet",
    copy: "Host I/O is how a Program talks to the machine",
  },
  {
    slug: "packages",
    title: "packages",
    status: "not-yet",
    copy: "Packages are git-backed.",
  },
] as const;

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

test("learn pages", () => {
  expect(statSync(pageDir).isDirectory()).toBe(true);
  for (const name of ["index.ts", "LearnPage.tsx", "LearnPage.types.ts"]) {
    expect(statSync(join(pageDir, name)).isFile()).toBe(true);
  }

  const pageSource = readFileSync(join(pageDir, "LearnPage.tsx"), "utf8");
  const routeTree = readFileSync(join(srcDir, "routeTree.gen.ts"), "utf8");
  const nav = readFileSync(
    join(srcDir, "features", "learn", "LearnNav", "LearnNav.tsx"),
    "utf8",
  );

  expect(pageSource).toContain("DocsShell");
  expect(pageSource).toContain("LearnNav");
  expect(pageSource).toContain("body");
  expect(pageSource).toContain("shipped");
  expect(pageSource).toContain("not-yet");
  expect(pageSource).not.toMatch(/playground/i);
  expect(pageSource).not.toContain("docs/specs");
  expect(pageSource).not.toContain(".html");
  expect(pageSource).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(pageSource).not.toMatch(/\bmax-w-sm\b/);

  expect(nav).not.toContain("@ts-expect-error");

  const learnRoute = readFileSync(join(routesDir, "learn.tsx"), "utf8");
  expect(learnRoute).toContain("createFileRoute");
  expect(learnRoute).toContain('"/learn"');
  expect(learnRoute).toContain('loadMarkdownPage("learn")');
  expect(routeTree).toContain("./routes/learn");
  expect(routeTree).toContain("'/learn'");

  for (const chapter of learnChapters) {
    const routePath = join(routesDir, `${chapter.slug}.tsx`);
    expect(statSync(routePath).isFile()).toBe(true);
    const route = readFileSync(routePath, "utf8");
    const source = readFileSync(join(websiteDir, `${chapter.slug}.md`), "utf8");
    const page = loadMarkdownPage(chapter.slug);
    const html = renderMarkdown(page.body);

    expect(route).toContain("createFileRoute");
    expect(route).toContain(`"/${chapter.slug}"`);
    expect(route).toContain(`loadMarkdownPage("${chapter.slug}")`);
    expect(route).toContain("LearnPage");
    expect(route).not.toContain("HomeHero");
    expect(route).not.toMatch(/playground/i);
    expect(route).not.toContain("docs/specs");
    expect(route).not.toContain(".html");
    expect(route).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(route).not.toMatch(/\bmax-w-sm\b/);

    expect(routeTree).toContain(`./routes/${chapter.slug}`);
    expect(routeTree).toContain(`'/${chapter.slug}'`);

    expect(page.title).toBe(chapter.title);
    expect(page.section).toBe("learn");
    expect(page.status).toBe(chapter.status);
    expect(page.body).toContain(`# ${chapter.title}`);
    expect(page.body).toContain(chapter.copy);
    expect(source).toContain(chapter.copy);
    expect(source).toContain(`status: ${chapter.status}`);

    expect(html).toContain(`<h1>${chapter.title}</h1>`);
    expect(html).toContain(chapter.copy);
    expect(html).not.toMatch(/href="[^"]*\.html"/);
    expect(html).not.toMatch(/playground/i);
    expect(html).not.toContain("docs/");
  }

  const fromJs = renderMarkdown(loadMarkdownPage("from-javascript").body);
  const fromSystems = renderMarkdown(loadMarkdownPage("from-systems").body);
  const dualWorlds = renderMarkdown(loadMarkdownPage("dual-worlds").body);
  const install = renderMarkdown(loadMarkdownPage("install").body);

  expect(fromJs).toContain('<a href="/dual-worlds">Dual worlds</a>');
  expect(fromSystems).toContain('<a href="/dual-worlds">Dual worlds</a>');
  expect(install).toContain('<a href="/from-javascript">from JavaScript</a>');
  expect(install).toContain('<a href="/from-systems">from systems</a>');
  expect(install).toContain("`node` on PATH");
  expect(dualWorlds).toContain('<a href="/from-javascript">from JavaScript</a>');
  expect(dualWorlds).toContain('<a href="/from-systems">from systems</a>');
  expect(dualWorlds).toContain('<a href="/modules">modules</a>');
  expect(dualWorlds).toContain('<a href="/native-types">native types</a>');
  expect(dualWorlds).toContain('<a href="/host-io">host I/O</a>');
  expect(dualWorlds).toContain('<a href="/packages">packages</a>');
  expect(dualWorlds).toContain(
    '<a href="/dual-world-rules">Dual-world rules</a>',
  );

  const extra = ["tutorial", "getting-started", "beginner", "vault"];
  for (const slug of extra) {
    expect(existsSync(join(routesDir, `${slug}.tsx`))).toBe(false);
  }

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
