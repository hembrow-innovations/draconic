import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { codeFenceVariants } from "./CodeFence.variants";

/**
 * Props for a copyable rendered code fence.
 */
export type CodeFenceProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof codeFenceVariants> & {
    code: string;
  };
