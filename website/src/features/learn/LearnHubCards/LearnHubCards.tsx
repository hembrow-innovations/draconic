import { Link } from "@tanstack/react-router";
import {
  learnHubCardTitleVariants,
  learnHubCardVariants,
  learnHubCardsVariants,
} from "./LearnHubCards.variants";
import type { LearnHubCardsProps } from "./LearnHubCards.types";

/**
 * Card grid of existing chapter links on `/learn`.
 *
 * Locks `public-site.ia:learn-walkable`. Hub markdown stays in DocsShell.
 * Two landings join at Dual worlds. Home landing stays a pitch, not this grid.
 *
 * @param props - Native div attributes
 * @returns Chapter card grid
 */
export function LearnHubCards({ className, ...props }: LearnHubCardsProps) {
  return (
    <div className={learnHubCardsVariants({ className })} {...props}>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/install" className={learnHubCardTitleVariants()}>Install</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/from-javascript" className={learnHubCardTitleVariants()}>from JavaScript</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/from-systems" className={learnHubCardTitleVariants()}>from systems</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/dual-worlds" className={learnHubCardTitleVariants()}>Dual worlds</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/modules" className={learnHubCardTitleVariants()}>modules</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/native-types" className={learnHubCardTitleVariants()}>native types</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/host-io" className={learnHubCardTitleVariants()}>host I/O</Link>
        </h3>
      </div>
      <div className={learnHubCardVariants()}>
        <h3>
          <Link to="/packages" className={learnHubCardTitleVariants()}>packages</Link>
        </h3>
      </div>
    </div>
  );
}
