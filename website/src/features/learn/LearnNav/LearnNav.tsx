import { Link } from "@tanstack/react-router";
import {
  learnNavLinkVariants,
  learnNavListVariants,
} from "./LearnNav.variants";
import type { LearnNavProps } from "./LearnNav.types";

/**
 * Learn aside sequence matching `website/learn.md`.
 *
 * Locks `public-site.ia:learn-walkable`. Two landings join at Dual worlds.
 *
 * @param props - Native list attributes
 * @returns Learn chapter list
 */
export function LearnNav({ className, ...props }: LearnNavProps) {
  return (
    <ul className={learnNavListVariants({ className })} {...props}>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/install" className={learnNavLinkVariants()}>Install</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/from-javascript" className={learnNavLinkVariants()}>from JavaScript</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/from-systems" className={learnNavLinkVariants()}>from systems</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/dual-worlds" className={learnNavLinkVariants()}>Dual worlds</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/modules" className={learnNavLinkVariants()}>modules</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/native-types" className={learnNavLinkVariants()}>native types</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/host-io" className={learnNavLinkVariants()}>host I/O</Link>
        }
      </li>
      <li>
        {
          // @ts-expect-error Learn chapter routes are registered in a later sitting
          <Link to="/packages" className={learnNavLinkVariants()}>packages</Link>
        }
      </li>
    </ul>
  );
}
