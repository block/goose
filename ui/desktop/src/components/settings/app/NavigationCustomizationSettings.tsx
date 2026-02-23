import React, { useState } from 'react';
import { GripVertical, Eye, EyeOff } from 'lucide-react';
import {
  useNavigationContext,
  DEFAULT_ITEM_ORDER,
  DEFAULT_ENABLED_ITEMS,
} from '../../Layout/NavigationContext';
import { cn } from '../../../utils';

const ITEM_LABELS: Record<string, string> = {
  home: 'Home',
  chat: 'Chat',
  recipes: 'Recipes',
  apps: 'Apps',
  scheduler: 'Scheduler',
  extensions: 'Extensions',
  settings: 'Settings',
};

interface NavigationCustomizationSettingsProps {
  className?: string;
}

export const NavigationCustomizationSettings: React.FC<NavigationCustomizationSettingsProps> = ({
  className,
}) => {
  const { preferences, updatePreferences } = useNavigationContext();
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);

  const handleDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, itemId: string) => {
    e.preventDefault();
    if (draggedItem && draggedItem !== itemId) {
      setDragOverItem(itemId);
    }
  };

  const handleDrop = (e: React.DragEvent, dropItemId: string) => {
    e.preventDefault();
    if (!draggedItem || draggedItem === dropItemId) return;

    const newOrder = [...preferences.itemOrder];
    const draggedIndex = newOrder.indexOf(draggedItem);
    const dropIndex = newOrder.indexOf(dropItemId);

    if (draggedIndex === -1 || dropIndex === -1) return;

    newOrder.splice(draggedIndex, 1);
    newOrder.splice(dropIndex, 0, draggedItem);

    updatePreferences({
      ...preferences,
      itemOrder: newOrder,
    });

    setDraggedItem(null);
    setDragOverItem(null);
  };

  const handleDragEnd = () => {
    setDraggedItem(null);
    setDragOverItem(null);
  };

  const toggleItemEnabled = (itemId: string) => {
    const newEnabledItems = preferences.enabledItems.includes(itemId)
      ? preferences.enabledItems.filter((id) => id !== itemId)
      : [...preferences.enabledItems, itemId];

    updatePreferences({
      ...preferences,
      enabledItems: newEnabledItems,
    });
  };

  const resetToDefaults = () => {
    updatePreferences({
      itemOrder: DEFAULT_ITEM_ORDER,
      enabledItems: DEFAULT_ENABLED_ITEMS,
    });
  };

  return (
    <div className={className}>
      <div className="space-y-3">
        <div className="flex items-center justify-between mb-4">
          <p className="text-sm text-text-secondary">
            Drag to reorder, click the eye icon to show/hide items
          </p>
          <button
            onClick={resetToDefaults}
            className="text-xs text-text-secondary hover:text-text-primary transition-colors"
          >
            Reset to defaults
          </button>
        </div>

        {preferences.itemOrder.map((itemId) => {
          const isEnabled = preferences.enabledItems.includes(itemId);
          const isDragging = draggedItem === itemId;
          const isDragOver = dragOverItem === itemId;
          const label = ITEM_LABELS[itemId] || itemId;

          return (
            <div
              key={itemId}
              draggable
              onDragStart={(e) => handleDragStart(e, itemId)}
              onDragOver={(e) => handleDragOver(e, itemId)}
              onDrop={(e) => handleDrop(e, itemId)}
              onDragEnd={handleDragEnd}
              className={cn(
                'flex items-center gap-3 p-3 rounded-lg border transition-all',
                isDragging && 'opacity-50',
                isDragOver
                  ? 'border-border-primary bg-background-tertiary'
                  : 'border-border-secondary bg-background-primary',
                !isEnabled && 'opacity-50'
              )}
            >
              <GripVertical className="w-4 h-4 text-text-secondary cursor-move flex-shrink-0" />
              <span className="flex-1 text-sm text-text-primary">{label}</span>
              <button
                onClick={() => toggleItemEnabled(itemId)}
                className="p-1 rounded hover:bg-background-tertiary transition-colors flex-shrink-0"
                title={isEnabled ? 'Hide item' : 'Show item'}
              >
                {isEnabled ? (
                  <Eye className="w-4 h-4 text-text-primary" />
                ) : (
                  <EyeOff className="w-4 h-4 text-text-secondary" />
                )}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};
