import { Link } from "@tanstack/react-router";
import {
  referenceNavLinkVariants,
  referenceNavListVariants,
} from "./ReferenceNav.variants";
import type { ReferenceNavProps } from "./ReferenceNav.types";

/**
 * Reference aside sequence matching `website/reference.md`.
 *
 * Locks `public-site.ia:reference-walkable`. Working pages kept open while writing a Program.
 *
 * @param props - Native list attributes
 * @returns Reference working-page list
 */
export function ReferenceNav({ className, ...props }: ReferenceNavProps) {
  return (
    <ul className={referenceNavListVariants({ className })} {...props}>
      <li>
        <Link to="/cli" className={referenceNavLinkVariants()}>CLI</Link>
      </li>
      <li>
        <Link to="/types" className={referenceNavLinkVariants()}>types</Link>
      </li>
      <li>
        <Link to="/dual-world-rules" className={referenceNavLinkVariants()}>Dual-world rules</Link>
      </li>
      <li>
        <Link to="/reference-host-io" className={referenceNavLinkVariants()}>host I/O</Link>
      </li>
      <li>
        <Link to="/reference-packages" className={referenceNavLinkVariants()}>packages</Link>
      </li>
    </ul>
  );
}
