import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const badgeVariants = cva(
  "inline-flex items-center rounded-full border border-stone-200 px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-stone-950 focus:ring-offset-2 dark:border-stone-800 dark:focus:ring-stone-300",
  {
    variants: {
      variant: {
        default:
          "border-transparent bg-stone-900 text-stone-50 hover:bg-stone-900/80 dark:bg-stone-50 dark:text-stone-900 dark:hover:bg-stone-50/80",
        secondary:
          "border-transparent bg-stone-100 text-stone-900 hover:bg-stone-100/80 dark:bg-stone-800 dark:text-stone-50 dark:hover:bg-stone-800/80",
        destructive:
          "border-transparent bg-red-500 text-stone-50 hover:bg-red-500/80 dark:bg-red-900 dark:text-stone-50 dark:hover:bg-red-900/80",
        outline: "text-stone-950 dark:text-stone-50",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  )
}

export { Badge, badgeVariants }
