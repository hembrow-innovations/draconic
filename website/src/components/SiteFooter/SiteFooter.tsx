import {
  siteFooterLinkVariants,
  siteFooterVariants,
} from "./SiteFooter.variants";
import type { SiteFooterProps } from "./SiteFooter.types";

/**
 * Quiet site chrome at the bottom of every page, not a Learn or Reference sitemap.
 *
 * @param props - Native footer attributes
 * @returns Site footer
 */
export function SiteFooter({ className, ...props }: SiteFooterProps) {
  return (
    <footer className={siteFooterVariants({ className })} {...props}>
      <p>Draconic</p>
      <a href="https://github.com/hembrow-innovations/draconic" className={siteFooterLinkVariants()}>GitHub</a>
    </footer>
  );
}
