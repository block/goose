import * as React from "react"
import { cn } from "../../utils"

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex h-9 w-full rounded-md border border-black/5 dark:border-white/5 bg-transparent px-3 py-1 text-base shadow-sm transition-colors",
          "file:border-0 file:bg-transparent file:text-sm file:font-medium",
          "text-goose-text dark:text-goose-text-dark",
          "placeholder:text-black/60 dark:placeholder:text-white/60",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-black/10 dark:focus-visible:ring-white/10",
          "disabled:cursor-not-allowed disabled:opacity-50",
          "md:text-sm",
          className
        )}
        ref={ref}
        {...props}
      />
    )
  }
)
Input.displayName = "Input"

export { Input }
