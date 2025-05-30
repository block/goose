import React from 'react';

import { motion } from 'framer-motion';

interface ChatDockProps {
  onTileCreatorToggle: () => void;
}

export const ChatDock: React.FC<ChatDockProps> = ({ onTileCreatorToggle }) => {
  return (
    <motion.div
      className="flex items-center gap-2 mb-2 px-2"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        delay: 0.2,
        type: 'spring',
        stiffness: 300,
        damping: 30,
      }}
    >
      <motion.button
        onClick={onTileCreatorToggle}
        className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-zinc-700/50 transition-colors"
        title="Toggle Tile Creator"
        whileHover={{ scale: 1.05 }}
        whileTap={{ scale: 0.95 }}
      >
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="text-gray-500 dark:text-gray-400"
        >
          <rect x="3" y="3" width="7" height="7" />
          <rect x="14" y="3" width="7" height="7" />
          <rect x="14" y="14" width="7" height="7" />
          <rect x="3" y="14" width="7" height="7" />
        </svg>
      </motion.button>
    </motion.div>
  );
};
