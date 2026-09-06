import {
  homeFeatureCardVariants,
  homeFeatureFactVariants,
  homeFeaturesVariants,
} from "./HomeFeatures.variants";
import type { HomeFeaturesProps } from "./HomeFeatures.types";

/**
 * Three language facts on `/`. No extra product claims.
 *
 * Locks `public-site.home:landing` together with the home hero.
 *
 * @param props - Native section attributes
 * @returns Home feature grid
 */
export function HomeFeatures({ className, ...props }: HomeFeaturesProps) {
  return (
    <section className={homeFeaturesVariants({ className })} {...props}>
      <article className={homeFeatureCardVariants()}>
        <p className={homeFeatureFactVariants()}>Compiles to JavaScript</p>
      </article>
      <article className={homeFeatureCardVariants()}>
        <p className={homeFeatureFactVariants()}>Compiles to native via LLVM</p>
      </article>
      <article className={homeFeatureCardVariants()}>
        <p className={homeFeatureFactVariants()}>
          Dual worlds are JS values and native types at explicit boundaries
        </p>
      </article>
    </section>
  );
}
