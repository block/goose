import React from 'react';
import { Play } from 'lucide-react';

interface ResumeButtonProps {
  onClick?: () => void; // Mark onClick as optional
  className?: string;
  textSize?: 'sm' | 'base' | 'md' | 'lg';
}

const ResumeButton: React.FC<ResumeButtonProps> = ({
  onClick,
  className = '',
  textSize = 'sm',
}) => {
  const handleExit = () => {
    if (onClick) {
      onClick(); // Custom onClick handler passed via props
    } else {
      console.warn('No history to go back to');
    }
  };

  return (
    <button
      onClick={handleExit}
      className={`flex items-center text-${textSize} text-textSubtle group hover:text-textStandard ${className}`}
    >
      <Play className="w-3 h-3 group-hover:-translate-x-1 transition-all mr-1" />
      <span>Resume</span>
    </button>
  );
};

export default ResumeButton;
