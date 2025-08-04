import React from 'react';
import { ArrowLeft } from 'lucide-react';
import { Button } from './button';
import type { VariantProps } from 'class-variance-authority';
import { buttonVariants } from './button';
import { cn } from '../../utils';

interface BackButtonProps extends VariantProps<typeof buttonVariants> {
  onClick?: () => void;
  className?: string;
  showText?: boolean;
  shape?: 'pill' | 'round';
}

const BackButton: React.FC<BackButtonProps> = ({
  onClick,
  className = '',
  variant = 'secondary',
  size = 'default',
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
      className={cn(
        'rounded-full px-6 py-2 flex items-center gap-2 text-text-default hover:cursor-pointer',
        className
      )}
      {...props}
    >
      <ArrowLeft />
      {showText && 'Back'}
    </Button>
  );
};

export default BackButton;
