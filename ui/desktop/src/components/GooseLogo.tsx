import { Avocado } from './icons/Avocado';
import { cn } from '../utils';

interface GooseLogoProps {
  className?: string;
  size?: 'default' | 'small';
  hover?: boolean;
}

/** Brand mark shown in loading / splash surfaces (legacy name kept for call sites). */
export default function GooseLogo({
  className = '',
  size = 'default',
  hover = true,
}: GooseLogoProps) {
  const sizes = {
    default: {
      frame: 'w-16 h-16',
      mark: 'w-16 h-16',
    },
    small: {
      frame: 'w-8 h-8',
      mark: 'w-8 h-8',
    },
  } as const;

  const currentSize = sizes[size];

  return (
    <div
      className={cn(
        className,
        currentSize.frame,
        'relative overflow-hidden flex items-center justify-center text-text-primary',
        hover && 'group/with-hover'
      )}
    >
      <Avocado
        className={cn(
          currentSize.mark,
          'transition-transform duration-300',
          hover && 'group-hover/with-hover:scale-105'
        )}
      />
    </div>
  );
}
