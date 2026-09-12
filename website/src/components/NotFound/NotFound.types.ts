import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { notFoundVariants } from "./NotFound.variants";

/**
 * Props for unknown-URL recovery. Chrome stays on the root route.
 */
export type NotFoundProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof notFoundVariants>;
