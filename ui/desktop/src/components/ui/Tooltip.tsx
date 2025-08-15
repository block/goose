import * as React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';

import { cn } from '../../utils';

// Create a global context to track tooltip provider instances
const TooltipContext = React.createContext<boolean>(false);

// Use this to check if we're inside a TooltipProvider
export function useTooltipContext() {
  return React.useContext(TooltipContext);
}

// Global provider that should be used at the root of your app
function TooltipProvider({
  delayDuration = 0,
  children,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
  // Check if we're already inside a provider
  const hasProvider = useTooltipContext();
  
  // If we already have a provider, just render children without creating another provider
  if (hasProvider) {
    return <>{children}</>;
  }
  
  // Otherwise create a new provider
  return (
    <TooltipContext.Provider value={true}>
      <TooltipPrimitive.Provider
        data-slot="tooltip-provider"
        delayDuration={delayDuration}
        {...props}
      >
        {children}
      </TooltipPrimitive.Provider>
    </TooltipContext.Provider>
  );
}

// Root tooltip component that ensures it has a provider
function Tooltip({ children, ...props }: React.ComponentProps<typeof TooltipPrimitive.Root>) {
  const hasProvider = useTooltipContext();
  
  // If we're already inside a provider, just render the root
  if (hasProvider) {
    return <TooltipPrimitive.Root data-slot="tooltip" {...props}>{children}</TooltipPrimitive.Root>;
  }
  
  // Otherwise wrap with a provider
  return (
    <TooltipProvider>
      <TooltipPrimitive.Root data-slot="tooltip" {...props}>
        {children}
      </TooltipPrimitive.Root>
    </TooltipProvider>
  );
}

function TooltipTrigger({ ...props }: React.ComponentProps<typeof TooltipPrimitive.Trigger>) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />;
}

function TooltipContent({
  className,
  sideOffset = 0,
  children,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        data-slot="tooltip-content"
        sideOffset={sideOffset}
        className={cn(
          'bg-background-accent text-text-on-accent animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 z-50 w-fit origin-(--radix-tooltip-content-transform-origin) rounded-md px-3 py-1.5 text-xs text-balance',
          className
        )}
        {...props}
      >
        {children}
        <TooltipPrimitive.Arrow className="bg-background-accent fill-background-accent z-50 size-2.5 translate-y-[calc(-50%_-_2px)] rotate-45" />
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  );
}

export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider };
