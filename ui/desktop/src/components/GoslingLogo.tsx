import { Gosling, Rain } from './icons/Gosling';
import { cn } from '../utils';

interface GoslingLogoProps {
  className?: string;
  size?: 'default' | 'small';
  hover?: boolean;
}

export default function GoslingLogo({
  className = '',
  size = 'default',
  hover = true,
}: GoslingLogoProps) {
  const sizes = {
    default: {
      frame: 'w-16 h-16',
      rain: 'w-[275px] h-[275px]',
      gosling: 'w-16 h-16',
    },
    small: {
      frame: 'w-8 h-8',
      rain: 'w-[150px] h-[150px]',
      gosling: 'w-8 h-8',
    },
  } as const;

  const currentSize = sizes[size];

  return (
    <div
      className={cn(
        className,
        currentSize.frame,
        'relative overflow-hidden',
        hover && 'group/with-hover'
      )}
    >
      <Rain
        className={cn(
          currentSize.rain,
          'absolute left-0 bottom-0 transition-all duration-300 z-1',
          hover && 'opacity-0 group-hover/with-hover:opacity-100'
        )}
      />
      <Gosling className={cn(currentSize.gosling, 'absolute left-0 bottom-0 z-2')} />
    </div>
  );
}
