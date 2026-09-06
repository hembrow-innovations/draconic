import { Link } from "@tanstack/react-router";
import { ThemeToggle } from "../ThemeToggle";
import {
  siteHeaderClusterVariants,
  siteHeaderLinkVariants,
  siteHeaderNavVariants,
  siteHeaderVariants,
  siteHeaderWordmarkVariants,
} from "./SiteHeader.variants";
import type { SiteHeaderProps } from "./SiteHeader.types";

/**
 * Primary site chrome for every page: wordmark, Learn, Reference, GitHub.
 *
 * Locks `public-site.chrome:primary-nav`. Chapter lists stay out of this header.
 *
 * @param props - Native header attributes
 * @returns Site header
 */
export function SiteHeader({ className, ...props }: SiteHeaderProps) {
  return (
    <header className={siteHeaderVariants({ className })} {...props}>
      <nav className={siteHeaderNavVariants()} aria-label="Primary">
        <Link to="/" className={siteHeaderWordmarkVariants()}>Draconic</Link>
        <div className={siteHeaderClusterVariants()}>
          {
            // @ts-expect-error Learn hub route is registered in a later sitting
            <Link to="/learn" className={siteHeaderLinkVariants()}>Learn</Link>
          }
          {
            // @ts-expect-error Reference hub route is registered in a later sitting
            <Link to="/reference" className={siteHeaderLinkVariants()}>Reference</Link>
          }
          <a href="https://github.com/hembrow-innovations/draconic" className={siteHeaderLinkVariants()}>GitHub</a>
          <ThemeToggle />
        </div>
      </nav>
    </header>
  );
}
