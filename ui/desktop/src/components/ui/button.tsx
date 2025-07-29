import * as React from 'react';
import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../utils';

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[1px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
  {
    variants: {
      variant: {
        default: 'bg-background-accent text-text-on-accent hover:bg-background-accent/90 shadow-xs',
        destructive:
          'bg-background-danger text-white hover:bg-background-danger/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-background-danger/60 shadow-xs',
        outline: 'border hover:bg-background-muted',
        secondary: 'bg-background-muted text-text-default hover:bg-background-muted/80 shadow-xs',
        ghost: 'hover:bg-background-muted dark:hover:bg-background-muted/50',
        link: 'text-text-accent underline-offset-4 hover:underline',
      },
      size: {
        xs: 'h-6 gap-1 ![&_svg:not([class*="size-"])]:size-3',
        default: 'h-9',
        sm: 'h-8 gap-1.5',
        lg: 'h-10',
      },
      shape: {
        pill: 'rounded-md',
        round: '',
      },
    },
    compoundVariants: [
      {
        shape: 'pill',
        size: 'xs',
        className: 'px-2 has-[>svg]:px-2',
      },
      {
        shape: 'pill',
        size: 'default',
        className: 'px-4 py-2 has-[>svg]:px-4',
      },
      {
        shape: 'pill',
        size: 'sm',
        className: 'px-4 has-[>svg]:px-3',
      },
      {
        shape: 'pill',
        size: 'lg',
        className: 'px-6 has-[>svg]:px-6',
      },
      {
        shape: 'round',
        size: 'xs',
        className: 'w-6 h-6 p-0 rounded-full',
      },
      {
        shape: 'round',
        size: 'default',
        className: 'w-9 h-9 p-0 rounded-full',
      },
      {
        shape: 'round',
        size: 'sm',
        className: 'w-8 h-8 p-0 rounded-full',
      },
      {
        shape: 'round',
        size: 'lg',
        className: 'w-10 h-10 p-0 rounded-full',
      },
    ],
    defaultVariants: {
      variant: 'default',
      size: 'default',
      shape: 'pill',
    },
  }
);

const Button = React.forwardRef<
  HTMLButtonElement,
  React.ComponentProps<'button'> &
    VariantProps<typeof buttonVariants> & {
      asChild?: boolean;
      shape?: 'pill' | 'round';
    }
>(({ className, variant, size, asChild = false, shape = 'pill', ...props }, ref) => {
  const Comp = asChild ? Slot : 'button';

  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, shape, className }))}
      ref={ref}
      {...props}
    />
  );
});

Button.displayName = 'Button';

export { Button, buttonVariants };
