import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");

test("markdown render install subset", () => {
  const page = loadMarkdownPage("install");
  const html = renderMarkdown(page.body);

  expect(html).toContain("<h1>Install</h1>");
  expect(html).toContain(
    '<h2 id="reproducibility"><a href="#reproducibility">Reproducibility</a></h2>',
  );
  expect(html).toContain(
    "<p>Get the toolchain, then parse, build, and run a Program before reading further.</p>",
  );
  expect(html).toContain("<ul>");
  expect(html).toContain("<li>linux/amd64</li>");
  expect(html).toContain("<li>windows/arm64</li>");
  expect(html).toContain("<pre>");
  expect(html).toContain("<code>");
  expect(html).toContain(
    "curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh",
  );
  expect(html).toContain(
    '<h2 id="from-source"><a href="#from-source">From source</a></h2>',
  );
  expect(html).toContain(
    '<h2 id="zed-editor"><a href="#zed-editor">Zed editor</a></h2>',
  );
  expect(html).toContain("cargo build -p draconic-cli --release");
  expect(html).toContain(
    '<a href="https://github.com/hembrow-innovations/draconic">repository README</a>',
  );
  expect(html).toContain("let console = globalThis.console;");
  expect(html).toContain("`node` on PATH");
  expect(html).toContain(
    '<a href="/from-javascript">from JavaScript</a>',
  );
  expect(html).toContain('<a href="/from-systems">from systems</a>');
  expect(html).toContain('<a href="/dual-worlds">Dual worlds</a>');

  const native = loadMarkdownPage("native-types");
  const nativeHtml = renderMarkdown(native.body);
  expect(nativeHtml).toContain(
    '<h2 id="i32-and-i64"><a href="#i32-and-i64">i32 and i64</a></h2>',
  );
  expect(nativeHtml).toContain(
    '<h2 id="fixed-structs"><a href="#fixed-structs">Fixed structs</a></h2>',
  );
  expect(nativeHtml).toContain("<h1>native types</h1>");

  expect(html).not.toContain("# Install");
  expect(html).not.toContain("## Reproducibility");
  expect(html).not.toContain("- linux/amd64");
  expect(html).not.toContain("[from JavaScript](from-javascript.html)");
  expect(html).not.toMatch(/playground/i);
  expect(html).not.toContain("docs/");
});

test("markdown render heading permalinks", () => {
  const html = renderMarkdown(loadMarkdownPage("install").body);
  const nativeHtml = renderMarkdown(loadMarkdownPage("native-types").body);
  const fromJsHtml = renderMarkdown(loadMarkdownPage("from-javascript").body);
  const packagesHtml = renderMarkdown(loadMarkdownPage("packages").body);
  const source = readFileSync(
    join(srcDir, "lib", "content", "renderMarkdown.ts"),
    "utf8",
  );

  expect(source).toContain("public-site.markdown:heading-permalinks");
  expect(html).toContain(
    '<h2 id="from-source"><a href="#from-source">From source</a></h2>',
  );
  expect(html).toContain(
    '<h2 id="reproducibility"><a href="#reproducibility">Reproducibility</a></h2>',
  );
  expect(html).toContain(
    '<h2 id="zed-editor"><a href="#zed-editor">Zed editor</a></h2>',
  );
  expect(html).toContain("<h1>Install</h1>");
  expect(html).not.toContain('<h1 id=');
  expect(html).not.toContain('<h1><a href=');
  expect(html).not.toContain('<h2 id="from-source">From source</h2>');
  expect(nativeHtml).toContain(
    '<h2 id="i32-and-i64"><a href="#i32-and-i64">i32 and i64</a></h2>',
  );
  expect(nativeHtml).toContain(
    '<h2 id="fixed-structs"><a href="#fixed-structs">Fixed structs</a></h2>',
  );
  expect(nativeHtml).toContain("<h1>native types</h1>");
  expect(fromJsHtml).toContain(
    '<h2 id="todo"><a href="#todo">Todo</a></h2>',
  );
  expect(fromJsHtml).toContain("<h1>from JavaScript</h1>");
  expect(packagesHtml).toContain(
    '<h2 id="flagship-service"><a href="#flagship-service">Flagship service</a></h2>',
  );
  expect(packagesHtml).toContain("<h1>packages</h1>");
  expect(source).not.toMatch(/playground/i);
  expect(source).not.toContain("docs/");
});

test("markdown render cli commands keep angle placeholders as text", () => {
  const page = loadMarkdownPage("cli");
  const html = renderMarkdown(page.body);

  expect(html).toContain("draconic parse &lt;file&gt;");
  expect(html).toContain("`node` on PATH");
  expect(html).toContain("[-o &lt;out&gt;]");
  expect(html).not.toContain("<file>");
  expect(html).not.toContain("<out>");
  expect(html).toContain('<a href="/reference-packages">packages</a>');
  expect(html).toContain("<pre>");
  expect(html).toContain("<code>");
  expect(html).toContain('id="parse"');
  expect(html).toContain('id="run"');
  expect(html).toContain('id="fmt"');
  expect(html).toContain('id="repl"');
});
