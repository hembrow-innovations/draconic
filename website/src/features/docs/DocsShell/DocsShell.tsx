import { Badge } from "../../../components/Badge";
import { renderMarkdown } from "../../../lib/content";
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
 * Handbook article chrome: kicker, shipped or not-yet Badge, and related-link footer.
 *
 * Locks `public-site.chrome:docs-sidebar` and `public-site.nav:learn-reference-status`.
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
  return (
    <div className={docsShellVariants({ className })} {...props}>
      <article className={docsShellArticleVariants()}>
        <p className={docsShellKickerVariants()}>{kicker}</p>
        <Badge variant={status}>{statusLabel(status)}</Badge>
        {body !== undefined ? (
          <div dangerouslySetInnerHTML={{ __html: renderMarkdown(body) }} />
        ) : null}
        {children ? (
          <footer className={docsShellFooterVariants()}>{children}</footer>
        ) : null}
      </article>
    </div>
  );
}
