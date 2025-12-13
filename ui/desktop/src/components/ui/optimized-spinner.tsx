import React from 'react';
import { Loader2 } from 'lucide-react';
import { cn } from '../../utils';

interface OptimizedSpinnerProps {
  className?: string;
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  color?: string;
  'aria-label'?: string;
}

// Size mappings
const sizeClasses = {
  xs: 'w-3 h-3',
  sm: 'w-4 h-4',
  md: 'w-6 h-6',
  lg: 'w-8 h-8',
  xl: 'w-10 h-10',
};

/**
 * GPU-accelerated spinner component for smooth performance
 * Uses CSS transforms and containment for optimal rendering
 */
export const OptimizedSpinner = React.memo<OptimizedSpinnerProps>(
  ({ className = '', size = 'sm', color, 'aria-label': ariaLabel = 'Loading' }) => {
    // Inject optimized keyframes on first mount
    React.useEffect(() => {
      const styleId = 'optimized-spin-keyframes';
      if (!document.getElementById(styleId)) {
        const style = document.createElement('style');
        style.id = styleId;
        style.textContent = `
        @keyframes optimized-spin {
          from {
            transform: rotate(0deg) translateZ(0);
          }
          to {
            transform: rotate(360deg) translateZ(0);
          }
        }
        
        .spinner-container {
          contain: layout style paint;
          will-change: transform;
          transform: translateZ(0);
          backface-visibility: hidden;
          perspective: 1000px;
        }
      `;
        document.head.appendChild(style);
      }
    }, []);

    return (
      <div
        className={cn('spinner-container', sizeClasses[size], className)}
        aria-label={ariaLabel}
        style={{
          animation: 'optimized-spin 1s linear infinite',
          color: color,
        }}
      >
        <Loader2 className="w-full h-full" />
      </div>
    );
  }
);

OptimizedSpinner.displayName = 'OptimizedSpinner';
