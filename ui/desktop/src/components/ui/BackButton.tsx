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
      className={`flex items-center text-sm text-textSubtle group hover:text-textStandard ${className}`}
    >
      <Back className="w-3 h-3 group-hover:-translate-x-1 transition-all mr-1" />
      <span>Exit</span>
    </button>
  );
};

export default BackButton;
