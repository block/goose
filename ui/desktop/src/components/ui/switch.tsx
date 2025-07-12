import * as React from 'react';
import * as SwitchPrimitives from '@radix-ui/react-switch';
import { cn } from '../../utils';

export const Switch = React.forwardRef<
  React.ElementRef<typeof SwitchPrimitives.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitives.Root> & {
    variant?: 'default' | 'mono';
  }
>(({ className, variant = 'default', ...props }, ref) => (
  <SwitchPrimitives.Root
    className={cn(
      'peer inline-flex h-[16px] w-[28px] shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50',
      variant === 'default'
        ? 'data-[state=checked]:bg-background-default data-[state=unchecked]:bg-input'
        : 'data-[state=checked]:bg-slate-900 dark:data-[state=checked]:bg-white data-[state=unchecked]:bg-slate-300 dark:data-[state=unchecked]:bg-slate-600',
      className
    )}
    {...props}
    ref={ref}
  >
    <SwitchPrimitives.Thumb
      className={cn(
        'pointer-events-none block h-3 w-3 rounded-full shadow-lg ring-0 transition-transform',
        variant === 'default'
          ? 'bg-background-default data-[state=checked]:translate-x-3 data-[state=unchecked]:translate-x-0'
          : 'bg-white dark:data-[state=checked]:bg-black dark:data-[state=unchecked]:bg-white data-[state=checked]:translate-x-3 data-[state=unchecked]:translate-x-0'
      )}
    />
  </SwitchPrimitives.Root>
));
Switch.displayName = SwitchPrimitives.Root.displayName;
