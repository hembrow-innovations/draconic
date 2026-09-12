import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const contentDir = join(websiteDir, "content");
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
    status: "shipped",
    copy: "This landing assumes you already write Rust, Go, or C.",
  },
  {
    slug: "dual-worlds",
    title: "Dual worlds",
    status: "shipped",
    copy: "Dual worlds is the coexistence of JS values and native types in one Program",
  },
  {
    slug: "modules",
    title: "modules",
    status: "shipped",
    copy: "A Program may be one file or an ECMAScript Module graph.",
  },
  {
    slug: "native-types",
    title: "native types",
    status: "shipped",
    copy: "A native type is a static, unboxed systems type.",
  },
  {
    slug: "host-io",
    title: "host I/O",
    status: "shipped",
    copy: "Host I/O is how a Program talks to the machine",
  },
  {
    slug: "packages",
    title: "packages",
    status: "shipped",
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
    const source = readFileSync(join(contentDir, `${chapter.slug}.md`), "utf8");
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

  const fromJsPage = loadMarkdownPage("from-javascript");
  const fromJs = renderMarkdown(fromJsPage.body);
  const fromSystems = renderMarkdown(loadMarkdownPage("from-systems").body);
  const dualWorlds = renderMarkdown(loadMarkdownPage("dual-worlds").body);
  const install = renderMarkdown(loadMarkdownPage("install").body);

  expect(fromJsPage.body).toContain("```drac");
  expect(fromJsPage.body).toContain("globalThis.console");
  expect(fromJsPage.body).toContain("unresolved");
  expect(fromJsPage.body).toContain("function greet(name: string): string");
  expect(fromJsPage.body).toContain("draconic check greet.drac");
  expect(fromJsPage.body.match(/```drac/g)?.length).toBe(2);
  expect(fromJs).toContain("greet");
  expect(fromJs).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/fizzbuzz">FizzBuzz</a>',
  );
  expect(fromJsPage.body).toContain("## Todo");
  expect(fromJsPage.body).toContain("`document`");
  expect(fromJsPage.body).toContain("`localStorage`");
  expect(fromJs).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/todo">Todo</a>',
  );
  expect(fromJs).toContain('<a href="/dual-worlds">Dual worlds</a>');
  const fromSystemsPage = loadMarkdownPage("from-systems");
  expect(fromSystemsPage.body).toContain("```drac");
  expect(fromSystemsPage.body).toContain("globalThis.console");
  expect(fromSystemsPage.body).toContain("function add(x: i32, y: i32): i32");
  expect(fromSystemsPage.body).toContain("as number");
  expect(fromSystemsPage.body).toContain("--target native");
  expect(fromSystemsPage.body).toContain("draconic check add.drac");
  expect(fromSystemsPage.body.match(/```drac/g)?.length).toBe(2);
  expect(fromSystems).toContain("add");
  expect(fromSystems).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/http-echo">HTTP echo</a>',
  );
  expect(fromSystems).toContain('<a href="/dual-worlds">Dual worlds</a>');
  expect(install).toContain('<a href="/from-javascript">from JavaScript</a>');
  expect(install).toContain('<a href="/from-systems">from systems</a>');
  expect(install).toContain("`node` on PATH");
  const installPage = loadMarkdownPage("install");
  const nodeIdx = installPage.body.indexOf("`node` on PATH");
  const runIdx = installPage.body.indexOf("draconic run hello.drac");
  expect(nodeIdx).toBeGreaterThan(-1);
  expect(runIdx).toBeGreaterThan(-1);
  expect(nodeIdx).toBeLessThan(runIdx);
  expect(installPage.body).toContain("## From source");
  expect(installPage.body).toContain("GitHub Releases");
  expect(installPage.body).toContain(
    "git clone https://github.com/hembrow-innovations/draconic.git",
  );
  expect(installPage.body).toContain("cargo build -p draconic-cli --release");
  expect(installPage.body).toContain("target/release/draconic");
  expect(installPage.body).not.toContain(
    "The clone-build-run path stays in the repository README.",
  );
  expect(install).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic">repository README</a>',
  );
  expect(installPage.body).toContain("## Zed editor");
  expect(installPage.body).toContain("`.drac`");
  expect(installPage.body).toContain("dev extension");
  expect(installPage.body).toContain("extension.toml");
  expect(installPage.body).toContain("draconic check");
  expect(installPage.body).toContain("draconic fmt");
  expect(installPage.body).not.toContain("language server");
  expect(installPage.body).not.toContain("LSP");
  expect(installPage.body).not.toContain("VS Code");
  expect(install).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/editors/zed">editors/zed</a>',
  );
  expect(install).toContain('<a href="/cli">CLI</a>');
  const zedIdx = installPage.body.indexOf("## Zed editor");
  const nativeIdx = installPage.body.indexOf("draconic build --target native hello.drac");
  const nextIdx = installPage.body.indexOf("Learn assumes you can already parse and build.");
  expect(zedIdx).toBeGreaterThan(-1);
  expect(nativeIdx).toBeGreaterThan(-1);
  expect(nextIdx).toBeGreaterThan(-1);
  expect(nativeIdx).toBeLessThan(zedIdx);
  expect(zedIdx).toBeLessThan(nextIdx);
  expect(dualWorlds).toContain('<a href="/from-javascript">from JavaScript</a>');
  expect(dualWorlds).toContain('<a href="/from-systems">from systems</a>');
  expect(dualWorlds).toContain('<a href="/modules">modules</a>');
  expect(dualWorlds).toContain('<a href="/native-types">native types</a>');
  expect(dualWorlds).toContain('<a href="/host-io">host I/O</a>');
  expect(dualWorlds).toContain('<a href="/packages">packages</a>');
  expect(dualWorlds).toContain(
    '<a href="/dual-world-rules">Dual-world rules</a>',
  );
  expect(loadMarkdownPage("dual-worlds").body).toContain("```drac");
  expect(loadMarkdownPage("dual-worlds").body).toContain("as i32");
  expect(loadMarkdownPage("dual-worlds").body).toContain("as number");
  expect(loadMarkdownPage("dual-worlds").body).toContain(
    "draconic check boundary.drac",
  );
  expect(dualWorlds).toContain("jsCount");
  expect(dualWorlds).toContain("<pre>");
  expect(dualWorlds).toContain("<code>");

  const nativeTypesPage = loadMarkdownPage("native-types");
  const nativeTypes = renderMarkdown(nativeTypesPage.body);
  expect(nativeTypesPage.body).toContain("```drac");
  expect(nativeTypesPage.body).toContain("let count: i32");
  expect(nativeTypesPage.body).toContain("let wide: i64");
  expect(nativeTypesPage.body).toContain("type Point = { x: i32; y: i32 }");
  expect(nativeTypesPage.body).toContain("as number");
  expect(nativeTypesPage.body).toContain("It builds today");
  expect(nativeTypesPage.body).toContain("draconic check width.drac");
  expect(nativeTypesPage.body).toContain("draconic check point.drac");
  expect(nativeTypesPage.body).toContain("## i32 and i64");
  expect(nativeTypesPage.body).toContain("## Fixed structs");
  expect(nativeTypesPage.body.match(/```drac/g)?.length).toBe(2);
  expect(nativeTypes).toContain("count");
  expect(nativeTypes).toContain("Point");
  expect(nativeTypes).toContain('<a href="/host-io">host I/O</a>');
  expect(nativeTypes).toContain('<a href="/types">types</a>');
  expect(nativeTypes).toContain("<pre>");
  expect(nativeTypes).toContain("<code>");

  const modulesPage = loadMarkdownPage("modules");
  const modulesHtml = renderMarkdown(modulesPage.body);
  expect(modulesPage.body).toContain("```drac");
  expect(modulesPage.body).toContain("export function greet");
  expect(modulesPage.body).toContain('from "./greet.drac"');
  expect(modulesPage.body).toContain("It builds today");
  expect(modulesPage.body).toContain("draconic check greet.drac");
  expect(modulesPage.body).toContain("## Entry");
  expect(modulesPage.body).toContain('console.log(greet("from the entry"));');
  expect(modulesPage.body).toContain("draconic check main.drac");
  expect(modulesPage.body).toContain("draconic run main.drac");
  expect(modulesPage.body.match(/```drac/g)?.length).toBe(1);
  expect(modulesHtml).toContain("greet");
  expect(modulesHtml).toContain('<a href="/native-types">native types</a>');
  expect(modulesHtml).toContain('<a href="/packages">packages</a>');
  expect(modulesHtml).toContain("<pre>");
  expect(modulesHtml).toContain("<code>");

  const hostIoPage = loadMarkdownPage("host-io");
  const hostIo = renderMarkdown(hostIoPage.body);
  expect(hostIoPage.body).toContain("```drac");
  expect(hostIoPage.body).toContain("stdoutWrite");
  expect(hostIoPage.body).toContain("writeFileText(");
  expect(hostIoPage.body).toContain("readFileText(");
  expect(hostIoPage.body).toContain("tcpListen");
  expect(hostIoPage.body).toContain("httpParseRequest");
  expect(hostIoPage.body).toContain("It builds today");
  expect(hostIoPage.body).toContain("draconic check hello-host.drac");
  expect(hostIoPage.body).toContain("draconic check note.drac");
  expect(hostIoPage.body).toContain("## tcpListen");
  expect(hostIoPage.body).toContain("tcpListen(8080)");
  expect(hostIoPage.body).toContain("closeTcp(s)");
  expect(hostIoPage.body).toContain("draconic check listen.drac");
  expect(hostIoPage.body).toContain(
    "draconic build --target native listen.drac -o listen",
  );
  expect(hostIoPage.body).toContain("## HTTP echo");
  expect(hostIoPage.body).toContain("tcpAccept(s)");
  expect(hostIoPage.body).toContain("tcpRead(a, 65536)");
  expect(hostIoPage.body).toContain("httpParseRequest(raw)");
  expect(hostIoPage.body).toContain("req.path");
  expect(hostIoPage.body).toContain(
    'httpWriteResponse(200, "OK", "Content-Type: text/plain\\r\\n", path)',
  );
  expect(hostIoPage.body).toContain("tcpWrite(a, resp)");
  expect(hostIoPage.body).toContain("while (true)");
  expect(hostIoPage.body).toContain("draconic check echo.drac");
  expect(hostIoPage.body).toContain(
    "draconic build --target native echo.drac -o echo",
  );
  expect(hostIoPage.body).toContain("free identifiers");
  expect(hostIoPage.body).not.toContain(
    "hard-errors unsupported host APIs until an explicit bridge exists",
  );
  expect(hostIoPage.body.match(/```drac/g)?.length).toBe(4);
  expect(hostIo).toContain("stdoutWrite");
  expect(hostIo).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/http-echo">HTTP echo</a>',
  );
  expect(hostIo).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/flagship-service">Flagship service</a>',
  );
  expect(hostIo).toContain('<a href="/packages">packages</a>');
  expect(hostIo).toContain('<a href="/reference-host-io">host I/O</a>');
  expect(hostIo).toContain('<a href="/cli">CLI</a>');
  expect(hostIo).toContain("<pre>");
  expect(hostIo).toContain("<code>");

  const packagesPage = loadMarkdownPage("packages");
  const packagesHtml = renderMarkdown(packagesPage.body);
  expect(packagesPage.body).toContain("```drac");
  expect(packagesPage.body).toContain("export function greet");
  expect(packagesPage.body).toContain('from "github.com/org/pkg"');
  expect(packagesPage.body).toContain("It builds today");
  expect(packagesPage.body).toContain("draconic check index.drac");
  expect(packagesPage.body).toContain("draconic get github.com/org/pkg@1.0.0");
  expect(packagesPage.body).toContain("draconic mod tidy");
  expect(packagesPage.body).toContain("--url");
  expect(packagesPage.body).toContain("## Package root");
  expect(packagesPage.body).toContain("## get");
  expect(packagesPage.body).toContain("## mod tidy");
  expect(packagesPage.body.match(/```drac/g)?.length).toBe(1);
  expect(packagesHtml).toContain("greet");
  expect(packagesHtml).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/pkg-lib">pkg-lib</a>',
  );
  expect(packagesHtml).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/pkg-consumer">pkg-consumer</a>',
  );
  expect(packagesPage.body).toContain("## Flagship service");
  expect(packagesHtml).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic/tree/main/examples/flagship-service">Flagship service</a>',
  );
  expect(packagesHtml).toContain('<a href="/modules">modules</a>');
  expect(packagesHtml).toContain('<a href="/reference-packages">packages</a>');
  expect(packagesHtml).toContain('<a href="/cli">CLI</a>');
  expect(packagesHtml).toContain("<pre>");
  expect(packagesHtml).toContain("<code>");

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
