import { forwardRef } from "react";
import { badgeVariants } from "./Badge.variants";
import type { BadgeProps } from "./Badge.types";

/**
 * Visible shipped or not-yet status chip. Classes come from Badge.variants, not JSX soup.
 *
 * @param props - Status variant plus native span attributes
 * @returns Status span
 */
export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(function Badge(
  { className, variant, children, ...props },
  ref,
) {
  return (
    <span
      ref={ref}
      className={badgeVariants({ variant, className })}
      {...props}
    >
      {children}
    </span>
  );
});
