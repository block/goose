import React from 'react';

import { platformService } from '@platform';

interface ButtonProps {
  onClick?: () => void;
  children: React.ReactNode;
  copyText?: string;
  variant?: 'primary' | 'secondary';
  className?: string;
}

export const Button: React.FC<ButtonProps> = ({
  onClick,
  children,
  copyText,
  variant = 'primary',
  className = '',
}) => {
  const handleClick = async () => {
    if (copyText) {
      try {
        console.log('Attempting to copy text:', copyText);
        await platformService.copyToClipboard(copyText);
        console.log('Text copied successfully');
      } catch (error) {
        console.error('Failed to copy:', error);
      }
    }

    if (onClick) {
      onClick();
    }
  };

  const getVariantStyles = () => {
    switch (variant) {
      case 'secondary':
        return {
          backgroundColor: '#6c757d',
          color: 'white',
        };
      case 'primary':
      default:
        return {
          backgroundColor: '#4CAF50',
          color: 'white',
        };
    }
  };

  return (
    <button
      className={`app-button ${className}`}
      onClick={handleClick}
      style={{
        padding: '10px 20px',
        border: 'none',
        borderRadius: '4px',
        cursor: 'pointer',
        transition: 'background-color 0.2s',
        ...getVariantStyles(),
      }}
    >
      {children}
    </button>
  );
};
