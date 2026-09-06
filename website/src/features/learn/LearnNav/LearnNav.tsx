import { Link } from "@tanstack/react-router";
import {
  learnNavLinkVariants,
  learnNavListVariants,
} from "./LearnNav.variants";
import type { LearnNavProps } from "./LearnNav.types";

const currentPage = { "aria-current": "page" as const };

/**
 * Learn sequence matching `website/learn.md`, grouped in the site side nav.
 *
 * Locks `public-site.ia:learn-walkable` and `public-site.nav:learn-reference-status`.
 * Two landings join at Dual worlds. The open page gets aria-current.
 *
 * @param props - Native list attributes
 * @returns Learn chapter list
 */
export function LearnNav({ className, ...props }: LearnNavProps) {
  return (
    <ul className={learnNavListVariants({ className })} {...props}>
      <li>
        <Link to="/install" className={learnNavLinkVariants()} activeProps={currentPage}>Install</Link>
      </li>
      <li>
        <Link to="/from-javascript" className={learnNavLinkVariants()} activeProps={currentPage}>from JavaScript</Link>
      </li>
      <li>
        <Link to="/from-systems" className={learnNavLinkVariants()} activeProps={currentPage}>from systems</Link>
      </li>
      <li>
        <Link to="/dual-worlds" className={learnNavLinkVariants()} activeProps={currentPage}>Dual worlds</Link>
      </li>
      <li>
        <Link to="/modules" className={learnNavLinkVariants()} activeProps={currentPage}>modules</Link>
      </li>
      <li>
        <Link to="/native-types" className={learnNavLinkVariants()} activeProps={currentPage}>native types</Link>
      </li>
      <li>
        <Link to="/host-io" className={learnNavLinkVariants()} activeProps={currentPage}>host I/O</Link>
      </li>
      <li>
        <Link to="/packages" className={learnNavLinkVariants()} activeProps={currentPage}>packages</Link>
      </li>
    </ul>
  );
}
