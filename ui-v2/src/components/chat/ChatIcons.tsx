import React, { useState } from 'react';

import { motion } from 'framer-motion';

import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';

// Define the tool items
const CHAT_TOOLS = [
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="white" stroke="none">
        <path d="M4 6h16v2H4zm0 5h16v2H4zm0 5h16v2H4z" />
      </svg>
    ),
    label: 'Make a Tile',
    color: 'bg-[#4F6BFF] hover:bg-[#4F6BFF]/90',
    rotation: -3,
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="white" stroke="none">
        <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 14l-5-5 1.41-1.41L12 14.17l4.59-4.58L18 11l-6 6z" />
      </svg>
    ),
    label: 'Tasks',
    color: 'bg-[#E042A5] hover:bg-[#E042A5]/90',
    rotation: 2,
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="white" stroke="none">
        <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
      </svg>
    ),
    label: 'Add',
    color: 'bg-[#05C168] hover:bg-[#05C168]/90',
    rotation: -2,
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="white" stroke="none">
        <path d="M12 2L1 21h22L12 2zm0 3.83L19.17 19H4.83L12 5.83zM11 16h2v2h-2zm0-6h2v4h-2z" />
      </svg>
    ),
    label: 'Issues',
    color: 'bg-[#FF9900] hover:bg-[#FF9900]/90',
    rotation: 3,
  },
];

interface ChatIconsProps {
  className?: string;
}

export const ChatIcons: React.FC<ChatIconsProps> = ({ className }) => {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  return (
    <div className={`flex mb-4 items-start ${className}`}>
      <div className="flex items-center justify-center">
        <div className="flex -space-x-6 relative">
          {CHAT_TOOLS.map((tool, index) => {
            const getX = () => {
              if (hoveredIndex === null) return 0;
              const spread = 16;
              const centerOffset = hoveredIndex * -spread;
              return index * spread + centerOffset;
            };

            return (
              <motion.div
                key={tool.label}
                className="relative"
                animate={{
                  x: getX(),
                  rotate: hoveredIndex !== null ? 0 : tool.rotation,
                  scale: hoveredIndex === index ? 1.1 : 1,
                  zIndex: hoveredIndex === index ? 10 : CHAT_TOOLS.length - index,
                }}
                transition={{
                  duration: 0.2,
                  ease: 'easeOut',
                  scale: { duration: 0.1 },
                }}
                onHoverStart={() => setHoveredIndex(index)}
                onHoverEnd={() => setHoveredIndex(null)}
              >
                <Tooltip>
                  <TooltipTrigger asChild>
                    <motion.button
                      aria-label={tool.label}
                      className={`
                        flex h-12 w-12 items-center justify-center rounded-xl 
                        transition-all duration-200 shadow-sm
                        ${tool.color}
                        ${hoveredIndex !== null && hoveredIndex !== index ? 'opacity-50' : ''}
                      `}
                    >
                      {tool.icon}
                    </motion.button>
                  </TooltipTrigger>
                  <TooltipContent sideOffset={5}>{tool.label}</TooltipContent>
                </Tooltip>
              </motion.div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
