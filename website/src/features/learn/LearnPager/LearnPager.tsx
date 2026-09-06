import { Link } from "@tanstack/react-router";
import { learnNeighbors } from "../learnSequence";
import {
  learnPagerGroupVariants,
  learnPagerLabelVariants,
  learnPagerLinkVariants,
  learnPagerVariants,
} from "./LearnPager.variants";
import type { LearnPagerProps } from "./LearnPager.types";

/**
 * Prev/next in the article footer. Locks `public-site.ia:learn-walkable`.
 *
 * Both landings continue to Dual worlds. They are not required in order.
 *
 * @param props - Chapter slug plus native nav attributes
 * @returns Sequence pager, or null when the slug is not a chapter
 */
export function LearnPager({ slug, className, ...props }: LearnPagerProps) {
  const { prev, next } = learnNeighbors(slug);
  if (prev.length === 0 && next.length === 0) {
    return null;
  }
  return (
    <nav
      aria-label="Learn sequence"
      className={learnPagerVariants({ className })}
      {...props}
    >
      <div className={learnPagerGroupVariants()}>
        {prev.length > 0 ? (
          <>
            <span className={learnPagerLabelVariants()}>Previous</span>
            {prev.map((stop) => (
              <Link
                key={stop.href}
                to={stop.href}
                className={learnPagerLinkVariants()}
              >
                {stop.label}
              </Link>
            ))}
          </>
        ) : null}
      </div>
      <div className={learnPagerGroupVariants()}>
        {next.length > 0 ? (
          <>
            <span className={learnPagerLabelVariants()}>Next</span>
            {next.map((stop) => (
              <Link
                key={stop.href}
                to={stop.href}
                className={learnPagerLinkVariants()}
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
