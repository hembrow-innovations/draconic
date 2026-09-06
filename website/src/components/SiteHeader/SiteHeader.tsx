import { useState, type KeyboardEvent } from "react";
import { Link } from "@tanstack/react-router";
import { LearnNav } from "../../features/learn/LearnNav";
import { ReferenceNav } from "../../features/reference/ReferenceNav";
import { SiteSearch } from "../SiteSearch";
import { ThemeToggle } from "../ThemeToggle";
import {
  siteHeaderClusterVariants,
  siteHeaderGroupVariants,
  siteHeaderLinkVariants,
  siteHeaderMenuButtonVariants,
  siteHeaderNavVariants,
  siteHeaderTaglineVariants,
  siteHeaderVariants,
  siteHeaderWordmarkVariants,
} from "./SiteHeader.variants";
import type { SiteHeaderProps } from "./SiteHeader.types";

const primaryNavId = "site-primary-nav";
const currentPage = { "aria-current": "page" as const };

/**
 * Sticky side nav for every page: wordmark, Learn pages, Reference pages, GitHub.
 *
 * Locks `public-site.chrome:odm-shell`, `public-site.chrome:primary-nav`,
 * `public-site.chrome:docs-sidebar`, `public-site.ia:learn-walkable`,
 * `public-site.ia:reference-walkable`, and `public-site.a11y:keyboard-small`.
 * Small viewports disclose the same primary links from a keyboard-operable button.
 *
 * @param props - Native aside attributes
 * @returns Site side nav
 */
export function SiteHeader({ className, ...props }: SiteHeaderProps) {
  const [open, setOpen] = useState(false);

  function onToggle() {
    setOpen((current) => !current);
  }

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape") {
      setOpen(false);
    }
  }

  return (
    <aside
      className={siteHeaderVariants({ className })}
      {...props}
      onKeyDown={onKeyDown}
    >
      <nav className={siteHeaderNavVariants()} aria-label="Primary">
        <Link to="/" className={siteHeaderWordmarkVariants()} activeProps={currentPage}>Draconic</Link>
        <p className={siteHeaderTaglineVariants()}>One language, two backends.</p>
        <button
          type="button"
          className={siteHeaderMenuButtonVariants()}
          aria-expanded={open}
          aria-controls={primaryNavId}
          onClick={onToggle}
        >Menu</button>
        <div
          id={primaryNavId}
          className={siteHeaderClusterVariants({ open })}
        >
          <div className={siteHeaderGroupVariants()}>
            <Link to="/learn" className={siteHeaderLinkVariants()} activeProps={currentPage}>Learn</Link>
            <LearnNav />
          </div>
          <div className={siteHeaderGroupVariants()}>
            <Link to="/reference" className={siteHeaderLinkVariants()} activeProps={currentPage}>Reference</Link>
            <ReferenceNav />
          </div>
          <a href="https://github.com/hembrow-innovations/draconic" className={siteHeaderLinkVariants()}>GitHub</a>
          <SiteSearch />
          <ThemeToggle />
        </div>
      </nav>
    </aside>
  );
}
