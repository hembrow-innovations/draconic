import { useState, type KeyboardEvent } from "react";
import { Link } from "@tanstack/react-router";
import { ThemeToggle } from "../ThemeToggle";
import {
  siteHeaderClusterVariants,
  siteHeaderLinkVariants,
  siteHeaderMenuButtonVariants,
  siteHeaderNavVariants,
  siteHeaderVariants,
  siteHeaderWordmarkVariants,
} from "./SiteHeader.variants";
import type { SiteHeaderProps } from "./SiteHeader.types";

const primaryNavId = "site-primary-nav";

/**
 * Primary site chrome for every page: wordmark, Learn, Reference, GitHub.
 *
 * Locks `public-site.chrome:primary-nav` and `public-site.a11y:keyboard-small`.
 * Small viewports disclose the same primary links from a keyboard-operable button.
 *
 * @param props - Native header attributes
 * @returns Site header
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
    <header
      className={siteHeaderVariants({ className })}
      {...props}
      onKeyDown={onKeyDown}
    >
      <nav className={siteHeaderNavVariants()} aria-label="Primary">
        <Link to="/" className={siteHeaderWordmarkVariants()}>Draconic</Link>
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
          <Link to="/learn" className={siteHeaderLinkVariants()}>Learn</Link>
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
