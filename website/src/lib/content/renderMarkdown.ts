const APP_SLUG = /^[a-z0-9-]+$/;

/**
 * Turn a markdown href into a Start path. Legacy `.html` links become `/slug`.
 *
 * @param href - Raw markdown href
 * @returns App path for in-site `.html` links; otherwise the original href
 */
export function toAppHref(href: string): string {
  if (
    /^[a-z][a-z0-9+.-]*:/i.test(href) ||
    href.startsWith("#") ||
    href.startsWith("/")
  ) {
    return href;
  }
  const hashAt = href.indexOf("#");
  const path = hashAt === -1 ? href : href.slice(0, hashAt);
  const hash = hashAt === -1 ? "" : href.slice(hashAt);
  if (!path.endsWith(".html")) {
    return href;
  }
  const slug = path.replace(/^\.\//, "").slice(0, -5);
  if (!APP_SLUG.test(slug)) {
    return href;
  }
  const base = (import.meta.env.BASE_URL ?? "/").replace(/\/$/, "");
  return `${base}/${slug}${hash}`;
}

/**
 * Turn the public markdown subset into HTML: headings, paragraphs, lists, fences, and links.
 *
 * @param body - Markdown body from a `website/` page, without frontmatter
 * @returns HTML for that subset
 */
export function renderMarkdown(body: string): string {
  const lines = body.split(/\r?\n/);
  let html = "";
  let inFence = false;
  let inList = false;

  for (const line of lines) {
    if (inFence) {
      if (line.startsWith("```")) {
        html += "</code></pre>\n";
        inFence = false;
      } else {
        html += `${line}\n`;
      }
      continue;
    }

    if (line.startsWith("```")) {
      if (inList) {
        html += "</ul>\n";
        inList = false;
      }
      html += "<pre><code>";
      inFence = true;
      continue;
    }

    if (line === "") {
      if (inList) {
        html += "</ul>\n";
        inList = false;
      }
      continue;
    }

    if (line.startsWith("#")) {
      if (inList) {
        html += "</ul>\n";
        inList = false;
      }
      let hashes = 0;
      while (hashes < line.length && line[hashes] === "#") {
        hashes += 1;
      }
      const text = line.slice(hashes).startsWith(" ")
        ? line.slice(hashes + 1)
        : line.slice(hashes);
      const level = hashes <= 3 ? hashes : 4;
      html += `<h${level}>${renderInline(text)}</h${level}>\n`;
      continue;
    }

    if (line.startsWith("- ")) {
      if (!inList) {
        html += "<ul>\n";
        inList = true;
      }
      html += `<li>${renderInline(line.slice(2))}</li>\n`;
      continue;
    }

    if (inList) {
      html += "</ul>\n";
      inList = false;
    }
    html += `<p>${renderInline(line)}</p>\n`;
  }

  if (inList) {
    html += "</ul>\n";
  }
  if (inFence) {
    html += "</code></pre>\n";
  }
  return html;
}

/**
 * Expand `[text](href)` links inside a heading, paragraph, or list item.
 *
 * @param text - One line of markdown inline content
 * @returns HTML with anchors for complete markdown links
 */
function renderInline(text: string): string {
  let html = "";
  let index = 0;
  while (index < text.length) {
    const char = text[index];
    if (char !== "[") {
      html += char;
      index += 1;
      continue;
    }
    index += 1;
    let label = "";
    while (index < text.length && text[index] !== "]") {
      label += text[index];
      index += 1;
    }
    if (index < text.length && text[index] === "]") {
      index += 1;
    }
    if (index < text.length && text[index] === "(") {
      index += 1;
      let href = "";
      while (index < text.length && text[index] !== ")") {
        href += text[index];
        index += 1;
      }
      if (index < text.length && text[index] === ")") {
        index += 1;
      }
      html += `<a href="${toAppHref(href)}">${label}</a>`;
    } else {
      html += `[${label}]`;
    }
  }
  return html;
}
