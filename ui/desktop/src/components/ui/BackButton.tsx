import React from 'react';
import { ChevronLeft } from 'lucide-react';
import { Button } from './button';
import type { VariantProps } from 'class-variance-authority';
import { buttonVariants } from './button';

interface BackButtonProps extends VariantProps<typeof buttonVariants> {
  onClick?: () => void;
  className?: string;
  showText?: boolean;
  shape?: 'pill' | 'round';
}

const BackButton: React.FC<BackButtonProps> = ({
  onClick,
  className = '',
  variant = 'outline',
  size = 'xs',
  shape = 'pill',
  showText = true,
  ...props
}) => {
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
    <Button
      onClick={handleExit}
      variant={variant}
      size={size}
      shape={shape}
      className={className}
      {...props}
    >
      <ChevronLeft />
      {showText && 'Back'}
    </Button>
  );
};

export default BackButton;
