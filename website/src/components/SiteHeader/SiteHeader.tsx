import { Link } from "@tanstack/react-router";
import { LearnNav } from "../../features/learn/LearnNav";
import { ReferenceNav } from "../../features/reference/ReferenceNav";
import { SiteSearch } from "../SiteSearch";
import { ThemeToggle } from "../ThemeToggle";
import {
  siteHeaderClusterVariants,
  siteHeaderGroupVariants,
  siteHeaderLinkVariants,
  siteHeaderNavVariants,
  siteHeaderTaglineVariants,
  siteHeaderVariants,
  siteHeaderWordmarkVariants,
} from "./SiteHeader.variants";
import type { SiteHeaderProps } from "./SiteHeader.types";

const currentPage = { "aria-current": "page" as const };

/**
 * Sticky side nav for every page: wordmark, Learn pages, Reference pages, GitHub.
 *
 * Locks `public-site.chrome:odm-shell`, `public-site.chrome:primary-nav`,
 * `public-site.chrome:current-page`, `public-site.search:titles-headings`,
 * `public-site.chrome:docs-sidebar`, `public-site.ia:learn-walkable`,
 * `public-site.ia:reference-walkable`, and `public-site.a11y:keyboard-small`.
 * Small viewports stack this nav above main and wrap its links; no hamburger.
 *
 * @param props - Native aside attributes
 * @returns Site side nav
 */
export function SiteHeader({ className, ...props }: SiteHeaderProps) {
  return (
    <aside className={siteHeaderVariants({ className })} {...props}>
      <nav className={siteHeaderNavVariants()} aria-label="Primary">
        <Link to="/" className={siteHeaderWordmarkVariants()} activeProps={currentPage}>Draconic</Link>
        <p className={siteHeaderTaglineVariants()}>One language, two backends.</p>
        <div className={siteHeaderClusterVariants()}>
          <SiteSearch />
          <ThemeToggle />
          <a href="https://github.com/hembrow-innovations/draconic" className={siteHeaderLinkVariants()}>GitHub</a>
          <div className={siteHeaderGroupVariants()}>
            <Link to="/learn" className={siteHeaderLinkVariants()} activeProps={currentPage}>Learn</Link>
            <LearnNav />
          </div>
          <div className={siteHeaderGroupVariants()}>
            <Link to="/reference" className={siteHeaderLinkVariants()} activeProps={currentPage}>Reference</Link>
            <ReferenceNav />
          </div>
        </div>
      </nav>
    </aside>
  );
}
