import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "./cn";

const badgeVariants = cva(
  "inline-flex items-center rounded-oak border px-2.5 py-0.5 text-xs font-medium transition-colors",
  {
    variants: {
      variant: {
        default:
          "border-oak-accent/30 bg-oak-accent/10 text-oak-accent",
        secondary:
          "border-oak-border bg-oak-bg-elevated text-oak-text-secondary",
        success:
          "border-oak-accent/30 bg-oak-accent/10 text-oak-accent",
        warning:
          "border-oak-warning/40 bg-oak-warning/10 text-oak-warning",
        destructive:
          "border-oak-error/40 bg-oak-error/10 text-oak-error",
        outline: "border-oak-border text-oak-text-primary",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

export { Badge, badgeVariants };
