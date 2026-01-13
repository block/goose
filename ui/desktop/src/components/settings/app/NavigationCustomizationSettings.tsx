import React, { useState, useCallback } from 'react';
import { GripVertical, Eye, EyeOff } from 'lucide-react';

export interface NavigationPreferences {
  itemOrder: string[];
  enabledItems: string[];
}

const DEFAULT_ITEM_ORDER = [
  'home',
  'chat',
  'history',
  'recipes',
  'scheduler',
  'extensions',
  'settings',
  'clock-widget',
  'activity-widget',
  'tokens-widget',
];

const DEFAULT_ENABLED_ITEMS = [...DEFAULT_ITEM_ORDER];

const ITEM_LABELS: Record<string, string> = {
  home: 'Home',
  chat: 'Chat',
  history: 'History',
  recipes: 'Recipes',
  scheduler: 'Scheduler',
  extensions: 'Extensions',
  settings: 'Settings',
  'clock-widget': 'Clock Widget',
  'activity-widget': 'Activity Widget',
  'tokens-widget': 'Tokens Widget',
};

export const useNavigationCustomization = () => {
  const [preferences, setPreferences] = useState<NavigationPreferences>(() => {
    const stored = localStorage.getItem('navigation_preferences');
    if (stored) {
      try {
        return JSON.parse(stored);
      } catch (e) {
        console.error('Failed to parse navigation preferences:', e);
      }
    }
    return {
      itemOrder: DEFAULT_ITEM_ORDER,
      enabledItems: DEFAULT_ENABLED_ITEMS,
    };
  });

  const updatePreferences = useCallback((newPreferences: NavigationPreferences) => {
    setPreferences(newPreferences);
    localStorage.setItem('navigation_preferences', JSON.stringify(newPreferences));
    window.dispatchEvent(
      new CustomEvent('navigation-preferences-updated', {
        detail: newPreferences,
      })
    );
  }, []);

  return { preferences, updatePreferences };
};

interface NavigationCustomizationSettingsProps {
  className?: string;
}

export const NavigationCustomizationSettings: React.FC<NavigationCustomizationSettingsProps> = ({
  className,
}) => {
  const { preferences, updatePreferences } = useNavigationCustomization();
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);

  const handleDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, itemId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
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
          <p className="text-sm text-text-muted">
            Drag to reorder, click the eye icon to show/hide items
          </p>
          <button
            onClick={resetToDefaults}
            className="text-xs text-text-muted hover:text-text-default transition-colors"
          >
            Reset to defaults
          </button>
        </div>

        {preferences.itemOrder.map((itemId) => {
          const isEnabled = preferences.enabledItems.includes(itemId);
          const isDragging = draggedItem === itemId;
          const isDragOver = dragOverItem === itemId;

          return (
            <div
              key={itemId}
              draggable
              onDragStart={(e) => handleDragStart(e, itemId)}
              onDragOver={(e) => handleDragOver(e, itemId)}
              onDrop={(e) => handleDrop(e, itemId)}
              onDragEnd={handleDragEnd}
              className={`
                flex items-center gap-3 p-3 rounded-lg border transition-all
                ${isDragging ? 'opacity-50' : 'opacity-100'}
                ${isDragOver ? 'border-border-strong bg-background-medium' : 'border-border-subtle bg-background-default'}
                ${!isEnabled ? 'opacity-50' : ''}
              `}
            >
              <GripVertical className="w-4 h-4 text-text-muted cursor-move" />
              <span className="flex-1 text-sm text-text-default">
                {ITEM_LABELS[itemId] || itemId}
              </span>
              <button
                onClick={() => toggleItemEnabled(itemId)}
                className="p-1 rounded hover:bg-background-medium transition-colors"
                title={isEnabled ? 'Hide item' : 'Show item'}
              >
                {isEnabled ? (
                  <Eye className="w-4 h-4 text-text-default" />
                ) : (
                  <EyeOff className="w-4 h-4 text-text-muted" />
                )}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};
