import { Link } from "@tanstack/react-router";
import {
  notFoundLinkVariants,
  notFoundTitleVariants,
  notFoundVariants,
} from "./NotFound.variants";
import type { NotFoundProps } from "./NotFound.types";

/**
 * Recovery main for an unknown URL: name the miss and offer a way back.
 *
 * Locks `public-site.chrome:not-found`. Skip-link, side nav, and footer stay on root.
 *
 * @param props - Native section attributes
 * @returns Not-found recovery
 */
export function NotFound({ className }: NotFoundProps) {
  return (
    <section className={notFoundVariants({ className })}>
      <title>Not found · Draconic</title>
      <h1 className={notFoundTitleVariants()}>Not found</h1>
      <Link to="/" className={notFoundLinkVariants()}>Home</Link>
    </section>
  );
}
