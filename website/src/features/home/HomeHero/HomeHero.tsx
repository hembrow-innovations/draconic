import { Link } from "@tanstack/react-router";
import {
  homeHeroCtaClusterVariants,
  homeHeroCtaVariants,
  homeHeroKickerVariants,
  homeHeroLeadVariants,
  homeHeroPathIndexVariants,
  homeHeroPathLinkVariants,
  homeHeroPathStepVariants,
  homeHeroPathVariants,
  homeHeroPitchVariants,
  homeHeroTitleVariants,
  homeHeroVariants,
} from "./HomeHero.variants";
import type { HomeHeroProps } from "./HomeHero.types";

/**
 * Language homepage pitch plus Install and Learn CTAs.
 *
 * Locks `public-site.home:landing`. The Learn hub article stays off `/`.
 *
 * @param props - Native section attributes
 * @returns Home hero
 */
export function HomeHero({ className, ...props }: HomeHeroProps) {
  return (
    <section className={homeHeroVariants({ className })} {...props}>
      <p className={homeHeroKickerVariants()}>Language</p>
      <h1 className={homeHeroTitleVariants()}>Draconic</h1>
      <p className={homeHeroLeadVariants()}>
        JavaScript you already know. Native types when you need them. One language, two backends.
      </p>
      <p className={homeHeroPitchVariants()}>
        A full ECMAScript superset with TypeScript-inspired static types and native systems types, compiling to JavaScript and to native binaries via LLVM.
      </p>
      <div className={homeHeroCtaClusterVariants()}>
        <Link to="/install" className={homeHeroCtaVariants({ variant: "primary" })}>Install</Link>
        <Link to="/learn" className={homeHeroCtaVariants({ variant: "ghost" })}>Learn</Link>
      </div>
      <ol className={homeHeroPathVariants()}>
        <li className={homeHeroPathStepVariants()}>
          <span className={homeHeroPathIndexVariants()}>1</span>
          <Link to="/install" className={homeHeroPathLinkVariants()}>Install</Link>
        </li>
        <li className={homeHeroPathStepVariants()}>
          <span className={homeHeroPathIndexVariants()}>2</span>
          <Link to="/learn" className={homeHeroPathLinkVariants()}>Learn</Link>
        </li>
        <li className={homeHeroPathStepVariants()}>
          <span className={homeHeroPathIndexVariants()}>3</span>
          <Link to="/reference" className={homeHeroPathLinkVariants()}>Reference</Link>
        </li>
      </ol>
    </section>
  );
}
