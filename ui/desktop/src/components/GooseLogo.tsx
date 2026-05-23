import { cn } from '../utils';
import apeCloudLogo from '../images/logo.png';

interface GooseLogoProps {
  className?: string;
  size?: 'default' | 'small';
  hover?: boolean;
}

export default function GooseLogo({
  className = '',
  size = 'default',
  hover = true,
}: GooseLogoProps) {
  const sizes = {
    default: {
      frame: 'w-16 h-16',
      goose: 'w-16 h-16',
    },
    small: {
      frame: 'w-8 h-8',
      goose: 'w-8 h-8',
    },
  } as const;

  const currentSize = sizes[size];

  return (
    <div
      className={cn(
        className,
        currentSize.frame,
        'relative overflow-hidden rounded-xl',
        hover && 'transition-transform duration-200 hover:scale-105'
      )}
    >
      <img
        src={apeCloudLogo}
        alt="ApeMind Agent"
        className={cn(currentSize.goose, 'absolute left-0 bottom-0 z-2')}
      />
    </div>
  );
}
