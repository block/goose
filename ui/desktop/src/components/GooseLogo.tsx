import React from 'react';
import { Goose, Rain } from './icons/Goose';

export default function GooseLogo({ className = '', size = 'default', hover = true }) {
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
    <div
      className={`${className} ${sizes[size].frame} ${hover ? 'group/with-hover' : ''} relative overflow-hidden`}
    >
      <Rain
        className={`${sizes[size].rain} absolute left-0 bottom-0 ${hover ? 'opacity-0 opacity-0 group-hover/with-hover:opacity-100' : ''} transition-all duration-300 z-1`}
      />
      <Goose className={`${sizes[size].goose} absolute left-0 bottom-0 z-2`} />
    </div>
  );
}
