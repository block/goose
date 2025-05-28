import { FC } from 'react';

import { Goose, Rain } from './icons/Goose';

interface GooseLogoProps {
  className?: string;
  size?: 'default' | 'small';
  hover?: boolean;
}

const GooseLogo: FC<GooseLogoProps> = ({ className = '', size = 'default', hover = true }) => {
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
  };

  return (
    <div className={`${className} ${sizes[size].frame} group relative`}>
      {/* Rain with enhanced visibility for testing */}
      <div
        className={`${sizes[size].rain} absolute left-0 bottom-0 ${hover ? 'opacity-0 group-hover:opacity-100' : 'opacity-100'} transition-all duration-500 z-10`}
        style={{
          filter: 'brightness(2) contrast(2) saturate(2)',
          mixBlendMode: 'multiply',
        }}
      >
        <Rain className="w-full h-full" />
      </div>
      <Goose className={`${sizes[size].goose} absolute left-0 bottom-0 z-20`} />
    </div>
  );
};

export default GooseLogo;
