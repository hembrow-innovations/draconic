import { Link } from "@tanstack/react-router";
import {
  referenceNavLinkVariants,
  referenceNavListVariants,
} from "./ReferenceNav.variants";
import type { ReferenceNavProps } from "./ReferenceNav.types";

const currentPage = { "aria-current": "page" as const };

/**
 * Reference sequence matching `website/reference.md`, grouped in the site side nav.
 *
 * Locks `public-site.ia:reference-walkable` and `public-site.nav:learn-reference-status`.
 * Working pages kept open while writing a Program. The open page gets aria-current.
 *
 * @param props - Native list attributes
 * @returns Reference working-page list
 */
export function ReferenceNav({ className, ...props }: ReferenceNavProps) {
  return (
    <ul className={referenceNavListVariants({ className })} {...props}>
      <li>
        <Link to="/cli" className={referenceNavLinkVariants()} activeProps={currentPage}>CLI</Link>
      </li>
      <li>
        <Link to="/types" className={referenceNavLinkVariants()} activeProps={currentPage}>types</Link>
      </li>
      <li>
        <Link to="/dual-world-rules" className={referenceNavLinkVariants()} activeProps={currentPage}>Dual-world rules</Link>
      </li>
      <li>
        <Link to="/reference-host-io" className={referenceNavLinkVariants()} activeProps={currentPage}>host I/O</Link>
      </li>
      <li>
        <Link to="/reference-packages" className={referenceNavLinkVariants()} activeProps={currentPage}>packages</Link>
      </li>
    </ul>
  );
}
