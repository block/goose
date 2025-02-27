// TooltipWrapper.tsx
import React from 'react';
import { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider } from '../../../../ui/Tooltip';
import { Portal } from '@radix-ui/react-portal';

interface TooltipWrapperProps {
  children: React.ReactNode;
  tooltipContent: React.ReactNode;
  side?: 'top' | 'bottom' | 'left' | 'right';
  align?: 'start' | 'center' | 'end';
  className?: string;
}

export function TooltipWrapper({
  children,
  tooltipContent,
  side = 'top',
  align = 'center',
  className = '',
}: TooltipWrapperProps) {
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>{children}</TooltipTrigger>
        <Portal>
          <TooltipContent side={side} align={align} className={className}>
            {typeof tooltipContent === 'string' ? <p>{tooltipContent}</p> : tooltipContent}
          </TooltipContent>
        </Portal>
      </Tooltip>
    </TooltipProvider>
  );
}
