import { Badge } from "../../../components/Badge";
import { CodeFence } from "../../../components/CodeFence";
import { renderMarkdown, splitMarkdownHtml } from "../../../lib/content";
import { OnThisPage } from "../OnThisPage";
import {
  docsShellArticleVariants,
  docsShellFooterVariants,
  docsShellKickerVariants,
  docsShellVariants,
} from "./DocsShell.variants";
import type { DocsShellProps } from "./DocsShell.types";

/**
 * Visible label for a teaching-page status chip.
 *
 * @param status - Frontmatter `shipped` or `not-yet`
 * @returns The same status word shown on the Badge
 */
function statusLabel(status: DocsShellProps["status"]): string {
  switch (status) {
    case "shipped":
      return "shipped";
    case "not-yet":
      return "not-yet";
  }
}

/**
 * Peel a leading h1 out of rendered article HTML so the badge can sit after the title.
 *
 * @param html - HTML from `renderMarkdown`
 * @returns Inner heading markup when the article starts with h1, plus the rest
 */
function takeLeadingH1(html: string): { heading: string | undefined; rest: string } {
  const open = "<h1>";
  const close = "</h1>";
  if (!html.startsWith(open)) {
    return { heading: undefined, rest: html };
  }
  const closeAt = html.indexOf(close);
  if (closeAt === -1) {
    return { heading: undefined, rest: html };
  }
  const heading = html.slice(open.length, closeAt);
  const after = html.slice(closeAt + close.length);
  const rest = after.startsWith("\n") ? after.slice(1) : after;
  return { heading, rest };
}

/**
 * Handbook article chrome: kicker, heading, shipped or not-yet Badge, on-page outline, and related-link footer.
 *
 * Locks `public-site.chrome:docs-sidebar`, `public-site.chrome:docs-article-order`,
 * `public-site.chrome:on-page-toc`, `public-site.nav:learn-reference-status`,
 * `public-site.fences:copy`, `public-site.fences:copy-announce`, and
 * `public-site.markdown:heading-permalinks`.
 * Section lists live in the site side nav. Home landing stays outside this shell.
 *
 * @param props - Section kicker, status from frontmatter, optional nav from callers, optional markdown body
 * @returns Docs shell
 */
export function DocsShell({
  className,
  kicker,
  status,
  nav: _nav,
  body,
  children,
  ...props
}: DocsShellProps) {
  const html = body !== undefined ? renderMarkdown(body) : undefined;
  const parts = html !== undefined ? takeLeadingH1(html) : undefined;
  return (
    <div className={docsShellVariants({ className })} {...props}>
      <article className={docsShellArticleVariants()}>
        <p className={docsShellKickerVariants()}>{kicker}</p>
        {parts?.heading !== undefined ? (
          <h1 dangerouslySetInnerHTML={{ __html: parts.heading }} />
        ) : null}
        <Badge variant={status}>{statusLabel(status)}</Badge>
        {body !== undefined ? <OnThisPage body={body} /> : null}
        {parts !== undefined && parts.rest !== ""
          ? splitMarkdownHtml(parts.rest).map((block, index) =>
              block.kind === "fence" ? (
                <CodeFence
                  key={index}
                  code={block.code}
                  label={`sample ${index + 1}`}
                />
              ) : (
                <div
                  key={index}
                  dangerouslySetInnerHTML={{ __html: block.html }}
                />
              ),
            )
          : null}
        {children ? (
          <footer className={docsShellFooterVariants()}>{children}</footer>
        ) : null}
      </article>
    </div>
  );
}
