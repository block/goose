import React from 'react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';

interface EnvironmentBadgeProps {
  className?: string;
}

const EnvironmentBadge: React.FC<EnvironmentBadgeProps> = ({ className = '' }) => {
  const isAlpha = process.env.ALPHA;
  const isDevelopment = import.meta.env.DEV;

  // Don't show badge in production
  if (!isDevelopment && !isAlpha) {
    return null;
  }

  const tooltipText = isAlpha ? 'Alpha' : 'Dev';
  const bgColor = isAlpha ? 'bg-purple-600' : 'bg-orange-400';

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          className={`${bgColor} w-2 h-2 rounded-full cursor-default ${className}`}
          data-testid="environment-badge"
          aria-label={tooltipText}
        />
      </TooltipTrigger>
      <TooltipContent side="right" portal={false}>{tooltipText}</TooltipContent>
    </Tooltip>
  );
};

export default EnvironmentBadge;
