import React from 'react';
import { X, Diamond } from 'lucide-react';

interface MentionPillProps {
  fileName: string;
  filePath: string;
  onRemove?: () => void; // Optional for read-only pills in messages
  variant?: 'default' | 'message'; // Different styles for input vs message display
  size?: 'sm' | 'md';
}

export const MentionPill: React.FC<MentionPillProps> = ({ 
  fileName, 
  filePath, 
  onRemove, 
  variant = 'default',
  size = 'sm'
}) => {
  const baseClasses = "inline-flex items-center gap-1.5 font-medium border rounded-full";
  
  const variantClasses = {
    default: "bg-orange-50 text-orange-700 border-orange-200 hover:bg-orange-100 dark:bg-orange-950 dark:text-orange-300 dark:border-orange-800 dark:hover:bg-orange-900",
    message: "bg-orange-100 text-orange-800 border-orange-200 dark:bg-orange-900 dark:text-orange-200 dark:border-orange-700"
  };
  
  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-3 py-1.5 text-sm"
  };

  return (
    <div 
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} transition-colors`}
      title={filePath} // Show full path on hover
    >
      <span className="flex items-center gap-1">
        <Diamond size={12} className="text-orange-500 fill-orange-500" />
        {fileName}
      </span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="flex items-center justify-center w-4 h-4 rounded-full hover:bg-orange-200 dark:hover:bg-orange-800 transition-colors"
          aria-label={`Remove ${fileName} mention`}
        >
          <X size={10} />
        </button>
      )}
    </div>
  );
};

export default MentionPill;
