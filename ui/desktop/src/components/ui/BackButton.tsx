import React from 'react';
import Back from '../icons/Back';

interface BackButtonProps {
  onClick?: () => void; // Mark onClick as optional
  className?: string;
}

const BackButton: React.FC<BackButtonProps> = ({ onClick, className = '' }) => {
  const handleExit = () => {
    if (onClick) {
      onClick(); // Custom onClick handler passed via props
    } else if (window.history.length > 1) {
      window.history.back(); // Navigate to the previous page
    } else {
      console.warn('No history to go back to');
    }
  };

  return (
    <button
      onClick={handleExit}
      className={`flex items-center gap-2 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 mb-16 mt-4 ${className}`}
    >
      <Back className="w-4 h-4" />
      <span>Back</span>
    </button>
  );
};

export default BackButton;
