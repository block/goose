import React, { useEffect, useCallback } from 'react';
import Back from '../icons/Back';

interface BackButtonProps {
  onClick?: () => void; // Mark onClick as optional
  className?: string;
  textSize?: 'sm' | 'base' | 'md' | 'lg';
  iconSize?: 'w-3 h-3' | 'w-4 h-4' | 'w-5 h-5' | 'w-6 h-6' | 'w-7 h-7';
  showText?: boolean; // Add new prop
}

const BackButton: React.FC<BackButtonProps> = ({
  onClick,
  className = '',
  textSize = 'sm',
  iconSize = 'w-3 h-3',
  showText = true,
}) => {
  const handleExit = useCallback(() => {
    if (onClick) {
      onClick(); // Custom onClick handler passed via props
    } else if (window.history.length > 1) {
      window.history.back(); // Navigate to the previous page
    } else {
      console.warn('No history to go back to');
    }
  }, [onClick]);

  // Set up mouse back button event listener.
  useEffect(() => {
    const handleMouseBack = () => {
      handleExit();
    };

    if (window.electron) {
      const mouseBackHandler = (e: MouseEvent) => {
        // MouseButton 3 or 4 is typically back button.
        if (e.button === 3 || e.button === 4) {
          handleExit();
          e.preventDefault();
        }
      };

      window.electron.on('mouse-back-button-clicked', handleMouseBack);

      // Also listen for mouseup events directly, for better OS compatibility.
      document.addEventListener('mouseup', mouseBackHandler);

      return () => {
        if (window.electron) {
          window.electron.off('mouse-back-button-clicked', handleMouseBack);
        }
        document.removeEventListener('mouseup', mouseBackHandler);
      };
    }

    return undefined;
  }, [handleExit]);

  return (
    <button
      onClick={handleExit}
      className={`flex items-center text-${textSize} text-textSubtle group hover:text-textStandard ${className}`}
    >
      <Back className={`${iconSize} group-hover:-translate-x-1 transition-all mr-1`} />
      {showText && <span>Back</span>}
    </button>
  );
};

export default BackButton;
