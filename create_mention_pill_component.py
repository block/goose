# Create MentionPill component similar to ActionPill
mention_pill_content = '''import React from 'react';
import { X, File } from 'lucide-react';

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
    default: "bg-green-100 text-green-800 border-green-200 dark:bg-green-900 dark:text-green-200 dark:border-green-700",
    message: "bg-green-100 text-green-800 border-green-200 dark:bg-green-900 dark:text-green-200 dark:border-green-700"
  };
  
  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-3 py-1.5 text-sm"
  };

  return (
    <div 
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]}`}
      title={filePath} // Show full path on hover
    >
      <span className="flex items-center gap-1">
        <File size={12} />
        {fileName}
      </span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="flex items-center justify-center w-4 h-4 rounded-full hover:bg-white/20 transition-colors"
          aria-label={`Remove ${fileName} mention`}
        >
          <X size={10} />
        </button>
      )}
    </div>
  );
};

export default MentionPill;
'''

# Write the MentionPill component
with open('ui/desktop/src/components/MentionPill.tsx', 'w') as f:
    f.write(mention_pill_content)

print("✅ Created MentionPill component:")
print("   - Similar structure to ActionPill")
print("   - Uses green colors to differentiate from actions (blue)")
print("   - Shows File icon and filename")
print("   - Tooltip shows full file path")
print("   - Supports removal and different variants")
