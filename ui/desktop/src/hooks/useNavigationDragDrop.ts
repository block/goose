import { useState, useCallback } from 'react';
import type { NavigationPreferences } from '../components/Layout/NavigationContext';

interface UseNavigationDragDropOptions {
  preferences: NavigationPreferences;
  updatePreferences: (prefs: NavigationPreferences) => void;
}

export function useNavigationDragDrop({
  preferences,
  updatePreferences,
}: UseNavigationDragDropOptions) {
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDragOver = useCallback(
    (e: React.DragEvent, itemId: string) => {
      e.preventDefault();
      if (draggedItem && draggedItem !== itemId) {
        setDragOverItem(itemId);
      }
    },
    [draggedItem]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent, dropItemId: string) => {
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
    },
    [draggedItem, preferences, updatePreferences]
  );

  const handleDragEnd = useCallback(() => {
    setDraggedItem(null);
    setDragOverItem(null);
  }, []);

  return {
    draggedItem,
    dragOverItem,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
  };
}
