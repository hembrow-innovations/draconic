import { Link } from "@tanstack/react-router";
import { referenceNeighbors } from "../referenceSequence";
import {
  referencePagerGroupVariants,
  referencePagerLabelVariants,
  referencePagerLinkVariants,
  referencePagerVariants,
} from "./ReferencePager.variants";
import type { ReferencePagerProps } from "./ReferencePager.types";

/**
 * Prev/next in the article footer. Locks `public-site.ia:reference-walkable`
 * and `public-site.chrome:docs-article-order`.
 *
 * Sequence is CLI, types, Dual-world rules, host I/O, packages.
 *
 * @param props - Working-page slug plus native nav attributes
 * @returns Sequence pager, or null when the slug is not a working page
 */
export function ReferencePager({
  slug,
  className,
  ...props
}: ReferencePagerProps) {
  const { prev, next } = referenceNeighbors(slug);
  if (prev.length === 0 && next.length === 0) {
    return null;
  }
  return (
    <nav
      aria-label="Reference sequence"
      className={referencePagerVariants({ className })}
      {...props}
    >
      <div className={referencePagerGroupVariants()}>
        {prev.length > 0 ? (
          <>
            <span className={referencePagerLabelVariants()}>Previous</span>
            {prev.map((stop) => (
              <Link
                key={stop.href}
                to={stop.href}
                className={referencePagerLinkVariants()}
              >
                {stop.label}
              </Link>
            ))}
          </>
        ) : null}
      </div>
      <div className={referencePagerGroupVariants()}>
        {next.length > 0 ? (
          <>
            <span className={referencePagerLabelVariants()}>Next</span>
            {next.map((stop) => (
              <Link
                key={stop.href}
                to={stop.href}
                className={referencePagerLinkVariants()}
              >
                {stop.label}
              </Link>
            ))}
          </>
        ) : null}
      </div>
    </nav>
  );
}
