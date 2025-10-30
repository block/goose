import React from 'react';
import { X } from 'lucide-react';

interface ActionPillProps {
  actionId: string;
  label: string;
  icon: React.ReactNode;
  onRemove?: () => void; // Optional for read-only pills in messages
  variant?: 'default' | 'message'; // Different styles for input vs message display
  size?: 'sm' | 'md';
}

export const ActionPill: React.FC<ActionPillProps> = ({ 
  label, 
  icon, 
  onRemove, 
  variant = 'default',
  size = 'sm'
}) => {
  const baseClasses = "inline-flex items-center gap-1.5 font-medium border rounded-full";
  
  const variantClasses = {
    default: "bg-blue-50 text-blue-700 border-blue-200 hover:bg-blue-100 dark:bg-blue-950 dark:text-blue-300 dark:border-blue-800 dark:hover:bg-blue-900",
    message: "bg-blue-100 text-blue-800 border-blue-200 dark:bg-blue-900 dark:text-blue-200 dark:border-blue-700"
  };
  
  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-3 py-1.5 text-sm"
  };

  return (
    <div className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} transition-colors`}>
      <span className="flex items-center gap-1">
        <span className="text-blue-500 flex items-center justify-center w-3 h-3">
          {icon}
        </span>
        {label}
      </span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="flex items-center justify-center w-4 h-4 rounded-full hover:bg-blue-200 dark:hover:bg-blue-800 transition-colors"
          aria-label={`Remove ${label} action`}
        >
          <X size={10} />
        </button>
      )}
    </div>
  );
};

export default ActionPill;
