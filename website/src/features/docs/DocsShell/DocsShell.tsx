import { Badge } from "../../../components/Badge";
import { renderMarkdown } from "../../../lib/content";
import {
  docsShellArticleVariants,
  docsShellAsideVariants,
  docsShellFooterVariants,
  docsShellKickerVariants,
  docsShellNavVariants,
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
 * Handbook chrome: section aside, article, and a shipped or not-yet Badge.
 *
 * Locks `public-site.chrome:docs-sidebar`. Home landing stays outside this shell.
 *
 * @param props - Section kicker, status from frontmatter, optional section nav, optional markdown body
 * @returns Docs shell
 */
export function DocsShell({
  className,
  kicker,
  status,
  nav,
  body,
  children,
  ...props
}: DocsShellProps) {
  return (
    <div className={docsShellVariants({ className })} {...props}>
      <aside className={docsShellAsideVariants()}>
        <nav className={docsShellNavVariants()} aria-label="Section">
          {nav}
        </nav>
      </aside>
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
