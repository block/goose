import type React from 'react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/atoms/tooltip';

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
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side} align={align} className={className}>
        {typeof tooltipContent === 'string' ? <p>{tooltipContent}</p> : tooltipContent}
      </TooltipContent>
    </Tooltip>
  );
}
