import React, { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { RotateCcw, Eye, EyeOff, Grip } from 'lucide-react';
import { Home, History, FileText, Puzzle, Settings as SettingsIcon, Users, Hash, ShoppingBag, Clock, Activity, BarChart3 } from 'lucide-react';
import { ChatSmart } from '../../icons';

// Define the navigation items that can be customized
export interface NavigationItem {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description: string;
  isCore?: boolean; // Core items cannot be disabled
  isWidget?: boolean;
}

// Default navigation items configuration
export const DEFAULT_NAVIGATION_ITEMS: NavigationItem[] = [
  {
    id: 'home',
    label: 'Home',
    icon: Home,
    description: 'Main dashboard and starting point',
    isCore: true,
  },
  {
    id: 'chat',
    label: 'Chat',
    icon: ChatSmart,
    description: 'AI conversation interface',
    isCore: true,
  },
  {
    id: 'history',
    label: 'History',
    icon: History,
    description: 'Previous chat sessions',
  },
  {
    id: 'recipes',
    label: 'Recipes',
    icon: FileText,
    description: 'Saved automation workflows',
  },
  {
    id: 'scheduler',
    label: 'Marketplace',
    icon: ShoppingBag,
    description: 'Browse and schedule recipes',
  },
  {
    id: 'extensions',
    label: 'Extensions',
    icon: Puzzle,
    description: 'Add-ons and integrations',
  },
  {
    id: 'peers',
    label: 'Peers',
    icon: Users,
    description: 'Collaborate with others',
  },
  {
    id: 'channels',
    label: 'Channels',
    icon: Hash,
    description: 'Team communication channels',
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: SettingsIcon,
    description: 'App configuration and preferences',
    isCore: true,
  },
  // Widget items
  {
    id: 'clock-widget',
    label: 'Clock Widget',
    icon: Clock,
    description: 'Analog clock display',
    isWidget: true,
  },
  {
    id: 'activity-widget',
    label: 'Activity Widget',
    icon: Activity,
    description: 'Session activity heatmap',
    isWidget: true,
  },
  {
    id: 'tokens-widget',
    label: 'Tokens Widget',
    icon: BarChart3,
    description: 'Token usage statistics',
    isWidget: true,
  },
];

// Navigation customization preferences type
export interface NavigationPreferences {
  enabledItems: string[];
  itemOrder: string[];
}

// Default preferences - all items enabled in default order
export const DEFAULT_NAVIGATION_PREFERENCES: NavigationPreferences = {
  enabledItems: DEFAULT_NAVIGATION_ITEMS.map(item => item.id),
  itemOrder: DEFAULT_NAVIGATION_ITEMS.map(item => item.id),
};

// Hook for managing navigation preferences
export function useNavigationCustomization() {
  const [preferences, setPreferences] = useState<NavigationPreferences>(DEFAULT_NAVIGATION_PREFERENCES);
  const [updateTrigger, setUpdateTrigger] = useState(0);

  // Load preferences from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem('navigation_preferences');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        setPreferences(parsed);
      } catch (error) {
        console.error('Failed to parse navigation preferences:', error);
        // Reset to defaults on error
        setPreferences(DEFAULT_NAVIGATION_PREFERENCES);
      }
    }
  }, []);

  // Listen for external preference updates (from other instances of the hook)
  useEffect(() => {
    const handleStorageChange = (event: CustomEvent) => {
      console.log('useNavigationCustomization: Received external update:', event.detail);
      setPreferences(event.detail);
      setUpdateTrigger(prev => prev + 1);
    };

    window.addEventListener('navigation-preferences-updated', handleStorageChange as EventListener);
    return () => {
      window.removeEventListener('navigation-preferences-updated', handleStorageChange as EventListener);
    };
  }, []);

  // Save preferences to localStorage whenever they change
  const updatePreferences = (newPreferences: NavigationPreferences) => {
    console.log('Updating navigation preferences:', newPreferences);
    setPreferences(newPreferences);
    localStorage.setItem('navigation_preferences', JSON.stringify(newPreferences));
    // Trigger custom event for navigation components to update
    const event = new CustomEvent('navigation-preferences-updated', { 
      detail: newPreferences 
    });
    console.log('Dispatching navigation-preferences-updated event:', event);
    window.dispatchEvent(event);
  };

  const toggleItem = (itemId: string) => {
    const item = DEFAULT_NAVIGATION_ITEMS.find(item => item.id === itemId);
    if (item?.isCore) return; // Cannot disable core items

    const newEnabledItems = preferences.enabledItems.includes(itemId)
      ? preferences.enabledItems.filter(id => id !== itemId)
      : [...preferences.enabledItems, itemId];

    updatePreferences({
      ...preferences,
      enabledItems: newEnabledItems,
    });
  };

  const resetToDefaults = () => {
    updatePreferences(DEFAULT_NAVIGATION_PREFERENCES);
  };

  const reorderItems = (newOrder: string[]) => {
    updatePreferences({
      ...preferences,
      itemOrder: newOrder,
    });
  };

  return {
    preferences,
    toggleItem,
    resetToDefaults,
    reorderItems,
  };
}

export default function NavigationCustomizationSettings() {
  const { preferences, toggleItem, resetToDefaults, reorderItems } = useNavigationCustomization();
  const [draggedItem, setDraggedItem] = useState<string | null>(null);

  // Separate navigation items and widgets for better organization
  const navigationItems = DEFAULT_NAVIGATION_ITEMS.filter(item => !item.isWidget);
  const widgetItems = DEFAULT_NAVIGATION_ITEMS.filter(item => item.isWidget);

  const handleDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const handleDrop = (e: React.DragEvent, dropItemId: string) => {
    e.preventDefault();
    if (!draggedItem || draggedItem === dropItemId) return;

    const newOrder = [...preferences.itemOrder];
    const draggedIndex = newOrder.indexOf(draggedItem);
    const dropIndex = newOrder.indexOf(dropItemId);

    // Remove dragged item and insert at new position
    newOrder.splice(draggedIndex, 1);
    newOrder.splice(dropIndex, 0, draggedItem);

    // Update preferences with new order
    reorderItems(newOrder);
    
    setDraggedItem(null);
  };

  const handleDragEnd = () => {
    setDraggedItem(null);
  };

  const isItemEnabled = (itemId: string) => preferences.enabledItems.includes(itemId);
  const enabledCount = preferences.enabledItems.length;
  const totalCount = DEFAULT_NAVIGATION_ITEMS.length;

  return (
    <Card className="rounded-lg">
      <CardHeader className="pb-0">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="mb-1">Navigation Items</CardTitle>
            <CardDescription>
              Choose which navigation items to show. Core items cannot be disabled.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={resetToDefaults}
            className="flex items-center gap-2"
          >
            <RotateCcw className="w-4 h-4" />
            Reset
          </Button>
        </div>
      </CardHeader>
      <CardContent className="pt-4 px-4 space-y-6">
        {/* Summary */}
        <div className="flex items-center justify-between p-3 bg-background-muted rounded-lg">
          <span className="text-sm text-text-muted">
            {enabledCount} of {totalCount} items enabled
          </span>
          <div className="flex items-center gap-2 text-xs text-text-muted">
            <Eye className="w-4 h-4" />
            <span>Enabled</span>
            <EyeOff className="w-4 h-4 ml-2" />
            <span>Disabled</span>
          </div>
        </div>

        {/* Navigation Items Section */}
        <div>
          <h4 className="text-sm font-medium text-text-default mb-3">Navigation Items</h4>
          <div className="space-y-2">
            {navigationItems.map((item) => {
              const IconComponent = item.icon;
              const enabled = isItemEnabled(item.id);
              const isDragging = draggedItem === item.id;

              return (
                <div
                  key={item.id}
                  draggable={!item.isCore}
                  onDragStart={(e) => !item.isCore && handleDragStart(e, item.id)}
                  onDragOver={handleDragOver}
                  onDrop={(e) => handleDrop(e, item.id)}
                  onDragEnd={handleDragEnd}
                  className={`
                    flex items-center justify-between p-3 rounded-lg border border-border-default
                    bg-background-default transition-all duration-200
                    ${isDragging ? 'opacity-50 scale-95' : ''}
                    ${!item.isCore ? 'cursor-move hover:shadow-sm' : ''}
                  `}
                  style={{
                    opacity: isDragging ? 0.5 : undefined,
                  }}
                >
                  <div className="flex items-center gap-3 flex-1">
                    {!item.isCore && (
                      <Grip className="w-4 h-4 text-text-muted" />
                    )}
                    <IconComponent className="w-5 h-5 text-text-default" />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-text-default">
                          {item.label}
                        </span>
                        {item.isCore && (
                          <span className="text-xs px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded-full">
                            Core
                          </span>
                        )}
                      </div>
                      <p className="text-xs mt-0.5 text-text-muted">
                        {item.description}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center">
                    <Switch
                      checked={enabled}
                      onCheckedChange={() => toggleItem(item.id)}
                      disabled={item.isCore}
                      variant="mono"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Widget Items Section */}
        <div>
          <h4 className="text-sm font-medium text-text-default mb-3">Widget Items</h4>
          <div className="space-y-2">
            {widgetItems.map((item) => {
              const IconComponent = item.icon;
              const enabled = isItemEnabled(item.id);
              const isDragging = draggedItem === item.id;

              return (
                <div
                  key={item.id}
                  draggable
                  onDragStart={(e) => handleDragStart(e, item.id)}
                  onDragOver={handleDragOver}
                  onDrop={(e) => handleDrop(e, item.id)}
                  onDragEnd={handleDragEnd}
                  className={`
                    flex items-center justify-between p-3 rounded-lg border border-border-default
                    bg-background-default transition-all duration-200 cursor-move hover:shadow-sm
                    ${isDragging ? 'opacity-50 scale-95' : ''}
                  `}
                  style={{
                    opacity: isDragging ? 0.5 : undefined,
                  }}
                >
                  <div className="flex items-center gap-3 flex-1">
                    <Grip className="w-4 h-4 text-text-muted" />
                    <IconComponent className="w-5 h-5 text-text-default" />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-text-default">
                          {item.label}
                        </span>
                        <span className="text-xs px-2 py-0.5 bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 rounded-full">
                          Widget
                        </span>
                      </div>
                      <p className="text-xs mt-0.5 text-text-muted">
                        {item.description}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center">
                    <Switch
                      checked={enabled}
                      onCheckedChange={() => toggleItem(item.id)}
                      variant="mono"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Info note */}
        <div className="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
          <p className="text-xs text-blue-700 dark:text-blue-300">
            <strong>Note:</strong> Changes apply to all navigation modes (tiles, condensed, overlay). 
            You can drag items to reorder them, and the order will be preserved across all navigation styles.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
