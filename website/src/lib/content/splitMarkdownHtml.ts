/**
 * One article chunk after fences are peeled out of rendered markdown HTML.
 */
export type MarkdownHtmlBlock =
  | { kind: "html"; html: string }
  | { kind: "fence"; code: string };

const FENCE = /<pre><code>([\s\S]*?)<\/code><\/pre>\n?/g;

/**
 * Split rendered markdown HTML into prose chunks and copyable fence bodies.
 *
 * @param html - Output of `renderMarkdown`
 * @returns Prose and fence blocks in document order
 */
export function splitMarkdownHtml(html: string): MarkdownHtmlBlock[] {
  const blocks: MarkdownHtmlBlock[] = [];
  let last = 0;
  for (const match of html.matchAll(FENCE)) {
    const start = match.index ?? 0;
    if (start > last) {
      blocks.push({ kind: "html", html: html.slice(last, start) });
    }
    blocks.push({ kind: "fence", code: decodeFence(match[1] ?? "") });
    last = start + match[0].length;
  }
  if (last < html.length) {
    blocks.push({ kind: "html", html: html.slice(last) });
  }
  return blocks;
}

function decodeFence(code: string): string {
  return code
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}
