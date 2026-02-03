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
          className={`relative cursor-default no-drag ${className}`}
          data-testid="environment-badge"
          aria-label={tooltipText}
        >
          <div className="absolute -inset-1" />
          <div className={`${bgColor} w-2 h-2 rounded-full`} />
        </div>
      </TooltipTrigger>
      <TooltipContent
        side="bottom"
        className={bgColor}
        arrowClassName={isAlpha ? 'fill-purple-600 bg-purple-600' : 'fill-orange-400 bg-orange-400'}
      >
        {tooltipText}
      </TooltipContent>
    </Tooltip>
  );
};

export default EnvironmentBadge;
