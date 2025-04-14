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
      'peer inline-flex h-[20px] w-[36px] shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50',
      variant === 'default'
        ? 'data-[state=checked]:bg-bgStandard data-[state=unchecked]:bg-input'
        : 'data-[state=checked]:bg-black dark:data-[state=checked]:bg-white data-[state=unchecked]:bg-bgStandard dark:data-[state=unchecked]:bg-[#333333]',
      className
    )}
    {...props}
    ref={ref}
  >
    <SwitchPrimitives.Thumb
      className={cn(
        'pointer-events-none block h-4 w-4 rounded-full shadow-lg ring-0 transition-transform',
        variant === 'default'
          ? 'bg-bgProminent data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0'
          : 'bg-white dark:bg-black data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0'
      )}
    />
  </SwitchPrimitives.Root>
));
Switch.displayName = SwitchPrimitives.Root.displayName;
