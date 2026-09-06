import { Link } from "@tanstack/react-router";
import {
  homeHeroCtaClusterVariants,
  homeHeroCtaVariants,
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
      <h1 className={homeHeroTitleVariants()}>Draconic</h1>
      <p className={homeHeroPitchVariants()}>
        JavaScript you already know. Native types when you need them. One language, two backends.
      </p>
      <p className={homeHeroPitchVariants()}>
        A full ECMAScript superset with TypeScript-inspired static types and native systems types, compiling to JavaScript and to native binaries via LLVM.
      </p>
      <div className={homeHeroCtaClusterVariants()}>
        {
          // @ts-expect-error Install route is registered in a later sitting
          <Link to="/install" className={homeHeroCtaVariants({ variant: "primary" })}>Install</Link>
        }
        {
          // @ts-expect-error Learn hub route is registered in a later sitting
          <Link to="/learn" className={homeHeroCtaVariants({ variant: "secondary" })}>Learn</Link>
        }
      </div>
    </section>
  );
}
