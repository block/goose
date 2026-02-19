import type React from 'react';

interface EnvironmentBadgeProps {
  className?: string;
}

const EnvironmentBadge: React.FC<EnvironmentBadgeProps> = ({ className = '' }) => {
  const isAlpha = process.env.ALPHA;
  const isDevelopment = import.meta.env.DEV;

  if (!isDevelopment && !isAlpha) {
    return null;
  }

  const label = isAlpha ? 'Alpha' : 'Dev';
  const bgColor = isAlpha ? 'bg-purple-600' : 'bg-orange-400';

  return (
    <div
      className={`${bgColor} w-2 h-2 rounded-full cursor-default ${className}`}
      data-testid="environment-badge"
      aria-label={label}
      title={label}
    />
  );
};

export default EnvironmentBadge;
