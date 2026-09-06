import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

test("markdown render install subset", () => {
  const page = loadMarkdownPage("install");
  const html = renderMarkdown(page.body);

  expect(html).toContain("<h1>Install</h1>");
  expect(html).toContain("<h2>Reproducibility</h2>");
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
  expect(html).toContain("let console = globalThis.console;");
  expect(html).toContain(
    '<a href="from-javascript.html">from JavaScript</a>',
  );
  expect(html).toContain('<a href="from-systems.html">from systems</a>');
  expect(html).toContain('<a href="dual-worlds.html">Dual worlds</a>');

  expect(html).not.toContain("# Install");
  expect(html).not.toContain("## Reproducibility");
  expect(html).not.toContain("- linux/amd64");
  expect(html).not.toContain("[from JavaScript](from-javascript.html)");
  expect(html).not.toMatch(/playground/i);
  expect(html).not.toContain("docs/");
});
