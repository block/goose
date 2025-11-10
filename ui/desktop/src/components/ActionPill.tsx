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
    default: "bg-bgProminent text-textProminentInverse border-borderProminent",
    message: "bg-blue-100 text-blue-800 border-blue-200 dark:bg-blue-900 dark:text-blue-200 dark:border-blue-700"
  };
  
  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-3 py-1.5 text-sm"
  };

  return (
    <div className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]}`}>
      <span className="flex items-center gap-1">
        <span className="relative">
          <div className="w-3 h-3 bg-blue-500 rounded-full absolute inset-0" />
          <span className="relative text-white text-[8px] flex items-center justify-center w-3 h-3">
            {icon}
          </span>
        </span>
        {label}
      </span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="flex items-center justify-center w-4 h-4 rounded-full hover:bg-white/20 transition-colors"
          aria-label={`Remove ${label} action`}
        >
          <X size={10} />
        </button>
      )}
    </div>
  );
};

export default ActionPill;
