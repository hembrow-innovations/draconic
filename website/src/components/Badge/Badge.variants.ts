import { cva } from "class-variance-authority";

/**
 * Shipped versus not-yet looks for the status Badge, painted with theme roles.
 */
export const badgeVariants = cva(
  "inline-flex items-center rounded-full px-2 py-0.5 font-body text-mono font-medium uppercase tracking-wider",
  {
    variants: {
      variant: {
        shipped: "bg-accent text-accent-foreground",
        "not-yet": "bg-line text-muted",
      },
    },
  },
);
