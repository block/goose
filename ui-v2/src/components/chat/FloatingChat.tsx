import React, { useState, useEffect } from 'react';

import { ChatIcons } from './ChatIcons';

interface FloatingChatProps {
  children: React.ReactNode;
}

export const FloatingChat: React.FC<FloatingChatProps> = ({ children }) => {
  const [isVisible, setIsVisible] = useState(false);
  const [isHovering, setIsHovering] = useState(false);

  // Create a debounced version of setIsVisible
  useEffect(() => {
    const timer = setTimeout(
      () => {
        setIsVisible(isHovering);
      },
      isHovering ? 0 : 200
    );

    return () => clearTimeout(timer);
  }, [isHovering]);

  return (
    <div
      className="fixed bottom-0 left-0 right-0 z-50"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      {/* Hover trigger area with black bar indicator */}
      <div className="absolute bottom-0 left-0 right-0 h-20 bg-transparent flex justify-center">
        <div
          className={`
            w-[600px] h-[15px]
            bg-black dark:bg-white
            rounded-t-[24px]
            transition-all duration-300
            absolute bottom-0
            ${isVisible ? 'opacity-0 transform translate-y-2' : 'opacity-100 transform translate-y-0'}
          `}
        />
      </div>

      {/* Chat container with transition */}
      <div
        className={`
          transform transition-all duration-300 ease-out
          ${isVisible ? 'translate-y-0 opacity-100' : '-translate-y-4 opacity-0'}
        `}
        style={{
          paddingBottom: 'env(safe-area-inset-bottom, 16px)',
        }}
      >
        <div className="flex justify-center w-full px-4 pb-4">
          <div className="w-[600px]">
            <ChatIcons className="mb-1" />
            {children}
          </div>
        </div>
      </div>
    </div>
  );
};
