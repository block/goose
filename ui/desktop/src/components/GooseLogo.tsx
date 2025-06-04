import { Goose, Rain } from './icons/Goose';

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
      rain: 'w-[275px] h-[275px]',
      goose: 'w-16 h-16',
    },
    small: {
      frame: 'w-8 h-8',
      rain: 'w-[150px] h-[150px]',
      goose: 'w-8 h-8',
    },
  } as const;

  const currentSize = sizes[size];

  return (
    <div
      className={`${className} ${currentSize.frame} ${hover ? 'group/with-hover' : ''} relative overflow-hidden`}
    >
      <Rain
        className={`${currentSize.rain} absolute left-0 bottom-0 ${hover ? 'opacity-0 group-hover/with-hover:opacity-100' : ''} transition-all duration-300 z-1`}
      />
      <Goose className={`${currentSize.goose} absolute left-0 bottom-0 z-2`} />
    </div>
  );
}
