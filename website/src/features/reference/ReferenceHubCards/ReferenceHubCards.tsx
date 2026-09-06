import { Link } from "@tanstack/react-router";
import {
  referenceHubCardTitleVariants,
  referenceHubCardVariants,
  referenceHubCardsVariants,
} from "./ReferenceHubCards.variants";
import type { ReferenceHubCardsProps } from "./ReferenceHubCards.types";

/**
 * Card grid of existing working-page links on `/reference`.
 *
 * Locks `public-site.ia:reference-walkable`. Hub markdown stays in DocsShell.
 * Home landing stays a pitch, not this grid.
 *
 * @param props - Native div attributes
 * @returns Working-page card grid
 */
export function ReferenceHubCards({
  className,
  ...props
}: ReferenceHubCardsProps) {
  return (
    <div className={referenceHubCardsVariants({ className })} {...props}>
      <div className={referenceHubCardVariants()}>
        <h3>
          <Link to="/cli" className={referenceHubCardTitleVariants()}>CLI</Link>
        </h3>
      </div>
      <div className={referenceHubCardVariants()}>
        <h3>
          <Link to="/types" className={referenceHubCardTitleVariants()}>types</Link>
        </h3>
      </div>
      <div className={referenceHubCardVariants()}>
        <h3>
          <Link to="/dual-world-rules" className={referenceHubCardTitleVariants()}>Dual-world rules</Link>
        </h3>
      </div>
      <div className={referenceHubCardVariants()}>
        <h3>
          <Link to="/reference-host-io" className={referenceHubCardTitleVariants()}>host I/O</Link>
        </h3>
      </div>
      <div className={referenceHubCardVariants()}>
        <h3>
          <Link to="/reference-packages" className={referenceHubCardTitleVariants()}>packages</Link>
        </h3>
      </div>
    </div>
  );
}
