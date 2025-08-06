import React, { useState, useEffect } from 'react';
import { IpcRendererEvent } from 'electron';
import { Hammer, Plus, Clock, FolderOpen } from 'lucide-react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
import { Card } from '../ui/card';
import { ScrollArea } from '../ui/scroll-area';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { toastSuccess, toastError } from '../../toasts';
import { Input } from '../ui/input';

interface AppTile {
  id: string;
  app_name: string;
  last_edited: number;
  path: string;
}

const BuildView: React.FC = () => {
  const [apps, setApps] = useState<AppTile[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [appName, setAppName] = useState('');
  const [creationProgress, setCreationProgress] = useState('');
  const [hasError, setHasError] = useState(false);
  const [colorPickerAppId, setColorPickerAppId] = useState<string | null>(null);
  const [appColors, setAppColors] = useState<Record<string, { bg: string; inner: string }>>({});
  
  // Available color combinations for app icons
  const colorCombinations = [
    { bg: 'bg-blue-100', inner: 'bg-blue-200', name: 'Blue' },
    { bg: 'bg-green-100', inner: 'bg-green-200', name: 'Green' },
    { bg: 'bg-red-100', inner: 'bg-red-200', name: 'Red' },
    { bg: 'bg-yellow-100', inner: 'bg-yellow-200', name: 'Yellow' },
    { bg: 'bg-purple-100', inner: 'bg-purple-200', name: 'Purple' },
    { bg: 'bg-pink-100', inner: 'bg-pink-200', name: 'Pink' },
    { bg: 'bg-background-medium', inner: 'bg-background-strong', name: 'Default' },
  ];

  const loadApps = async () => {
    try {
      const appsList = await window.electron.listApps();
      setApps(appsList);
    } catch (err) {
      console.error('Failed to load apps:', err);
      toastError({
        title: 'Failed to load apps',
        msg: 'Could not load apps from ~/goose/apps directory',
      });
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadApps();
    loadAppColors();

    // Listen for app creation progress
    const handleProgress = (_event: IpcRendererEvent, ...args: unknown[]) => {
      const data = args[0] as { appName: string; lastLine: string; type: 'stdout' | 'stderr' };
      setCreationProgress(data.lastLine);
      // Mark if we're getting stderr (potential errors)
      if (data.type === 'stderr') {
        setHasError(true);
      }
    };

    window.electron.on('app-creation-progress', handleProgress);

    // Cleanup listener
    return () => {
      window.electron.off('app-creation-progress', handleProgress);
    };
  }, []);

  const loadAppColors = async () => {
    try {
      const savedColors = await window.electron.loadAppColors();
      setAppColors(savedColors);
    } catch (err) {
      console.error('Failed to load app colors:', err);
    }
  };

  const handleCreateAppClick = () => {
    setAppName('');
    setShowCreateDialog(true);
  };

  const handleCreateApp = async () => {
    if (!appName.trim()) return;

    // Validate app name
    if (!/^[a-zA-Z0-9-_]+$/.test(appName.trim())) {
      toastError({
        title: 'Invalid app name',
        msg: 'App name can only contain letters, numbers, hyphens, and underscores',
      });
      return;
    }

    setIsCreating(true);
    setCreationProgress('Starting app creation...');
    setHasError(false);

    try {
      await window.electron.createApp(appName.trim());

      toastSuccess({
        title: 'App created!',
        msg: `Successfully created ${appName.trim()}`,
      });
      // Reload the apps list
      await loadApps();
      // Close dialog and reset
      setShowCreateDialog(false);
      setAppName('');
      setCreationProgress('');
      setHasError(false);
    } catch (err) {
      console.error('[BuildView] Failed to create app:', err);
      setHasError(true);
      setCreationProgress(err instanceof Error ? err.message : 'Unknown error occurred');
      toastError({
        title: 'Failed to create app',
        msg: err instanceof Error ? err.message : 'Unknown error occurred',
      });
    } finally {
      setIsCreating(false);
    }
  };

  const handleCancelCreate = () => {
    if (!isCreating) {
      setShowCreateDialog(false);
      setAppName('');
      setCreationProgress('');
      setHasError(false);
    }
  };

  const handleAppClick = async (app: AppTile) => {
    try {
      await window.electron.openApp(app.path);
    } catch (err) {
      console.error('Failed to open app:', err);
      toastError({
        title: 'Failed to open app',
        msg: 'Could not open the app directory',
      });
    }
  };

  // Component for rendering individual app tiles (similar to SessionItem)
  const AppTile: React.FC<{ app: AppTile }> = ({ app }) => {
    return (
      <Card
        className="h-full py-3 px-4 flex flex-col justify-between cursor-pointer hover:bg-background-medium/50 transition-all duration-200"
        onClick={() => handleAppClick(app)}
      >
        <div className="flex-1">
          {/* App name - matching session title exactly */}
          <h3 className="text-base truncate mb-1">{app.app_name}</h3>

          {/* Timestamp - similar to session */}
          <div className="flex items-center text-text-muted text-xs mb-1">
            <Clock className="w-3 h-3 mr-1 flex-shrink-0" />
            <span>{formatMessageTimestamp(app.last_edited)}</span>
          </div>

          {/* Path */}
          <div className="flex items-center text-text-muted text-xs mb-1">
            <FolderOpen className="w-3 h-3 mr-1 flex-shrink-0" />
            <span className="truncate">{app.path}</span>
          </div>
        </div>
      </Card>
    );
  };

  return (
    <MainPanelLayout>
      <div className="bg-background-muted flex flex-col h-full">
        {/* Header container with rounded bottom - matching home page */}
        <div className="bg-background-default rounded-b-2xl mb-0.5">
          <div className="px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Build</h1>
                {/* Create App button on the right - only show when apps exist (not in empty state) */}
                {!isLoading && apps.length > 0 && (
                  <Button
                    onClick={handleCreateAppClick}
                    variant="default"
                    className="flex items-center gap-2"
                    disabled={isCreating}
                  >
                    <Plus className="w-4 h-4" />
                    Create a new web app
                  </Button>
                )}
              </div>
              <p className="text-text-muted text-sm mt-2">
                Develop a goose hosted web app that you can share and create in the open.
              </p>
            </div>
          </div>
        </div>

        {/* Main content area with card-based layout - matching home page structure */}
        <div className="flex flex-col flex-1 space-y-0.5">
          {isLoading ? (
            /* Loading state - single container */
            <div className="bg-background-default rounded-2xl flex-1 py-6 px-6">
              <div className="flex items-center justify-center h-full text-text-muted">
                <div className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-text-muted"></div>
                  <span>Loading apps...</span>
                </div>
              </div>
            </div>
          ) : apps.length > 0 ? (
            /* Apps exist - individual containers for each app */
            <>
              {apps.map((app) => (
                <div key={app.id} className="bg-background-default rounded-2xl py-6 px-6">
                  <div className="flex flex-col h-full text-text-muted page-transition">
                    <div className="flex flex-col items-start">
                      {/* App image placeholder - 32x32 rounded square */}
                      <div 
                        className={`w-8 h-8 ${appColors[app.id]?.bg || 'bg-background-medium'} rounded-md mb-3 flex items-center justify-center cursor-pointer hover:opacity-80 transition-opacity duration-200`}
                        onClick={(e) => {
                          e.stopPropagation(); // Prevent triggering the app click
                          setColorPickerAppId(app.id);
                        }}
                      >
                        <div className={`w-4 h-4 ${appColors[app.id]?.inner || 'bg-background-strong'} rounded-sm`}></div>
                      </div>
                      
                      <h3 className="text-base truncate mb-1 text-text-default">{app.app_name}</h3>
                      
                      {/* Timestamp */}
                      <div className="flex items-center text-text-muted text-xs mb-1">
                        <Clock className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span>{formatMessageTimestamp(app.last_edited)}</span>
                      </div>

                      {/* Path */}
                      <div className="flex items-center text-text-muted text-xs mb-4">
                        <FolderOpen className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span className="truncate">{app.path}</span>
                      </div>
                    </div>
                    
                    <div className="flex justify-end mt-auto pt-6">
                      <Button
                        onClick={() => handleAppClick(app)}
                        variant="secondary"
                        className="flex items-center gap-2"
                      >
                        <FolderOpen className="w-4 h-4" />
                        Open App
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
              
              {/* Filler container - extends to fill remaining space */}
              <div className="bg-background-default rounded-2xl flex-1"></div>
            </>
          ) : (
            /* Empty state - two separate containers */
            <>
              {/* Empty state content container */}
              <div className="bg-background-default rounded-2xl py-6 px-6">
                <div className="flex flex-col h-full text-text-muted page-transition">
                  <div className="flex flex-col items-start">
                    <Hammer className="h-4 w-4 mb-4 text-inverse" />
                    <p className="text-sm">Your web apps will show up here. Create a new web app to get started in build.</p>
                  </div>
                  <div className="flex justify-end mt-auto pt-6">
                    <Button
                      onClick={handleCreateAppClick}
                      variant="default"
                      className="flex items-center gap-2"
                      disabled={isCreating}
                    >
                      <Plus className="w-4 h-4" />
                      Create your first web app
                    </Button>
                  </div>
                </div>
              </div>
              
              {/* Empty container stretching the full height */}
              <div className="bg-background-default rounded-2xl flex-1"></div>
            </>
          )}
        </div>
      </div>

      {/* Color Picker Dialog */}
      {colorPickerAppId && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[320px] max-w-[90vw]">
            <h3 className="text-lg font-medium text-text-standard mb-4">Choose App Icon Color</h3>

            <div className="grid grid-cols-4 gap-3 mb-6">
              {colorCombinations.map((combo, index) => (
                <div
                  key={index}
                  className="flex flex-col items-center cursor-pointer group"
                  onClick={async () => {
                    // Save color selection for the app
                    const newColors = { bg: combo.bg, inner: combo.inner };
                    setAppColors(prev => ({
                      ...prev,
                      [colorPickerAppId]: newColors
                    }));
                    
                    // Persist to storage
                    try {
                      await window.electron.saveAppColor(colorPickerAppId, newColors);
                    } catch (err) {
                      console.error('Failed to save app color:', err);
                    }
                    
                    setColorPickerAppId(null);
                  }}
                >
                  <div className={`w-8 h-8 ${combo.bg} rounded-md mb-2 flex items-center justify-center group-hover:scale-110 transition-transform duration-200`}>
                    <div className={`w-4 h-4 ${combo.inner} rounded-sm`}></div>
                  </div>
                  <span className="text-xs text-text-muted">{combo.name}</span>
                </div>
              ))}
            </div>

            <div className="flex justify-end">
              <Button 
                onClick={() => setColorPickerAppId(null)} 
                variant="ghost"
              >
                Cancel
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Create App Dialog */}
      {showCreateDialog && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[400px] max-w-[90vw]">
            <h3 className="text-lg font-medium text-text-standard mb-4">Create New App</h3>

            <div className="space-y-4">
              <div>
                <label
                  htmlFor="app-name"
                  className="block text-sm font-medium text-text-standard mb-2"
                >
                  App Name
                </label>
                <Input
                  id="app-name"
                  type="text"
                  value={appName}
                  onChange={(e) => setAppName(e.target.value)}
                  placeholder="my-awesome-app"
                  className="w-full"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && appName.trim()) {
                      handleCreateApp();
                    } else if (e.key === 'Escape') {
                      handleCancelCreate();
                    }
                  }}
                />
                <p className="text-xs text-text-muted mt-1">
                  Only letters, numbers, hyphens, and underscores are allowed
                </p>

                {/* Progress indicator */}
                {isCreating && (
                  <div className="mt-3 p-3 bg-background-muted rounded-lg border border-border-subtle">
                    <div className="flex items-center gap-2 mb-2">
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500"></div>
                      <span className="text-sm font-medium text-text-standard">
                        Creating app...
                      </span>
                    </div>
                    {creationProgress && (
                      <div
                        className={`text-xs font-mono bg-background-default p-2 rounded border ${
                          hasError ? 'text-red-600 border-red-300' : 'text-text-muted'
                        }`}
                      >
                        {creationProgress}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <Button onClick={handleCancelCreate} variant="ghost" disabled={isCreating}>
                Cancel
              </Button>
              <Button
                onClick={handleCreateApp}
                disabled={!appName.trim() || isCreating}
                variant="default"
                className="min-w-[120px]"
              >
                {isCreating ? (
                  <div className="flex items-center gap-2">
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                    Creating...
                  </div>
                ) : (
                  'Create App'
                )}
              </Button>
            </div>
          </div>
        </div>
      )}
    </MainPanelLayout>
  );
};

export default BuildView;
