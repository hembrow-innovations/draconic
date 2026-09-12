import { Link } from "@tanstack/react-router";
import {
  homeSampleKickerVariants,
  homeSampleLeadVariants,
  homeSampleLinkVariants,
  homeSamplePreVariants,
  homeSampleVariants,
} from "./HomeSample.variants";
import type { HomeSampleProps } from "./HomeSample.types";

/**
 * Static hello Program on `/`. Not a runner.
 *
 * Locks `public-site.home:landing` by showing source a visitor can write after Install.
 *
 * @param props - Native section attributes
 * @returns Home sample
 */
export function HomeSample({ className, ...props }: HomeSampleProps) {
  return (
    <section className={homeSampleVariants({ className })} {...props}>
      <p className={homeSampleKickerVariants()}>A Program</p>
      <p className={homeSampleLeadVariants()}>
        Save as hello.drac. It builds today.
      </p>
      <pre className={homeSamplePreVariants()}>
        <code>{`let console = globalThis.console;
console.log("hello from Draconic");
`}</code>
      </pre>
      <p className={homeSampleLeadVariants()}>
        <Link to="/install" className={homeSampleLinkVariants()}>
          Install
        </Link>
        {" to parse, build, and run it."}
      </p>
    </section>
  );
}
