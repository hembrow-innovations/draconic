import { skipLinkVariants } from "./SkipLink.variants";
import type { SkipLinkProps } from "./SkipLink.types";

/**
 * First keyboard stop: jump past chrome into the main landmark.
 *
 * Locks `public-site.chrome:odm-shell`. Partial lock of
 * `public-site.a11y:keyboard-small` (disclosure lives on the side nav).
 *
 * @param props - Native anchor attributes
 * @returns Skip link
 */
export function SkipLink({ className, ...props }: SkipLinkProps) {
  return (
    <a className={skipLinkVariants({ className })} {...props} href="#main">
      Skip to content
    </a>
  );
}
