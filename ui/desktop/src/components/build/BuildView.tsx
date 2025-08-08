import React, { useState, useEffect } from 'react';
import { IpcRendererEvent } from 'electron';
import { Hammer, Plus, Clock, FolderOpen, Globe } from 'lucide-react';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Button } from '../ui/button';
// import { Card } from '../ui/card';
// import { ScrollArea } from '../ui/scroll-area';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { toastSuccess, toastError } from '../../toasts';
import { Input } from '../ui/input';
import { getApiUrl } from '../../config';
import { ChatSmart } from '../icons';

interface AppTile {
  id: string;
  app_name: string;
  last_edited: number;
  path: string;
  subdomain?: string;
}

interface CreatingApp {
  id: string;
  displayName: string;
  appName: string;
  subdomain?: string;
  progress: string;
  log: string[];
  hasError: boolean;
  startTime: number;
}

const BuildView: React.FC = () => {
  // Domain configuration - easily changeable
  const DOMAIN = '.vibeplatstage.squarecdn.com';

  const [apps, setApps] = useState<AppTile[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [creatingApps, setCreatingApps] = useState<CreatingApp[]>([]);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [appName, setAppName] = useState('');
  const [displayAppName, setDisplayAppName] = useState(''); // What the user types
  const [subdomain, setSubdomain] = useState('');
  const [isCheckingSubdomain, setIsCheckingSubdomain] = useState(false);
  const [subdomainAvailable, setSubdomainAvailable] = useState<boolean | null>(null);
  const [subdomainCheckError, setSubdomainCheckError] = useState<string | null>(null);
  const [canConnectToSites, setCanConnectToSites] = useState(true);
  const [colorPickerAppId, setColorPickerAppId] = useState<string | null>(null);
  const [appColors, setAppColors] = useState<Record<string, { bg: string; inner: string }>>({});

  // Helper function to convert user input to a valid app name
  const convertToValidAppName = (input: string): string => {
    return input
      .toLowerCase()
      .trim()
      .replace(/[^a-zA-Z0-9\s-_]/g, '') // Remove invalid characters but keep spaces
      .replace(/\s+/g, '-') // Replace spaces with hyphens
      .replace(/-+/g, '-') // Replace multiple hyphens with single hyphen
      .replace(/^-|-$/g, ''); // Remove leading/trailing hyphens
  };

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
      
      // Update the specific creating app's progress
      setCreatingApps(prev => prev.map(app => 
        app.appName === data.appName 
          ? {
              ...app,
              progress: data.lastLine,
              log: [...app.log, data.lastLine],
              hasError: app.hasError || data.type === 'stderr'
            }
          : app
      ));
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

  const handleCreateAppClick = async () => {
    setAppName('');
    setDisplayAppName('');
    setSubdomain('');
    setSubdomainAvailable(null);
    setSubdomainCheckError(null);
    setShowCreateDialog(true);

    // Check if we can connect to the sites service using a placeholder subdomain
    // We use a placeholder that's unlikely to be available to test the connection
    try {
      const result = (await window.electron.ipcRenderer.invoke('check-subdomain', 'test')) as {
        available?: boolean;
        error?: boolean;
        message?: string;
      };
      console.log('[BuildView] Connection test result:', result);

      // Check if we got an error response
      if (result.error) {
        console.error('[BuildView] Cannot connect to sites service:', result.message);
        setCanConnectToSites(false);
      } else {
        // If we got a valid response (regardless of availability), the service is working
        setCanConnectToSites(true);
      }
    } catch (err) {
      console.error('[BuildView] Cannot connect to sites service:', err);
      // Log the full error for debugging
      if (err instanceof Error) {
        console.error('[BuildView] Error details:', {
          message: err.message,
          stack: err.stack,
          name: err.name,
        });
      }
      setCanConnectToSites(false);
    }
  };

  const checkSubdomainAvailability = async (siteName: string) => {
    if (!siteName || !canConnectToSites) return;

    setIsCheckingSubdomain(true);
    setSubdomainCheckError(null);

    try {
      const result = (await window.electron.ipcRenderer.invoke('check-subdomain', siteName)) as {
        available?: boolean;
        error?: boolean;
        message?: string;
      };

      // Check if we got an error response
      if (result.error) {
        console.error('[BuildView] Subdomain check error:', result.message);
        setSubdomainCheckError(result.message || 'Cannot connect to service');
        setSubdomainAvailable(false);
      } else if (result.available) {
        setSubdomainAvailable(true);
      } else {
        setSubdomainAvailable(false);
      }
    } catch (err) {
      console.error('Failed to check subdomain:', err);
      setSubdomainCheckError('Cannot connect to service');
    } finally {
      setIsCheckingSubdomain(false);
    }
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

    // If subdomain is provided and we can connect, check if it's available
    if (subdomain.trim() && canConnectToSites && !subdomainAvailable) {
      toastError({
        title: 'Subdomain not available',
        msg: 'Please choose a different subdomain',
      });
      return;
    }

    // Create a placeholder app in the creating apps list
    const creatingAppId = `creating-${Date.now()}`;
    const newCreatingApp: CreatingApp = {
      id: creatingAppId,
      displayName: displayAppName || appName,
      appName: appName.trim(),
      subdomain: subdomain.trim() || undefined,
      progress: 'Starting app creation...',
      log: [],
      hasError: false,
      startTime: Date.now()
    };

    setCreatingApps(prev => [...prev, newCreatingApp]);

    // Close dialog and reset form
    setShowCreateDialog(false);
    setAppName('');
    setDisplayAppName('');
    setSubdomain('');
    setSubdomainAvailable(null);
    setSubdomainCheckError(null);

    try {
      // The createApp function in main process will handle the path construction
      // We just pass the app name and subdomain
      await window.electron.createApp(newCreatingApp.appName, newCreatingApp.subdomain);

      // If subdomain was provided and available, claim it
      if (newCreatingApp.subdomain && canConnectToSites && subdomainAvailable) {
        try {
          // Update progress for subdomain claiming
          setCreatingApps(prev => prev.map(app => 
            app.id === creatingAppId 
              ? { ...app, progress: 'Claiming subdomain...' }
              : app
          ));
          
          await window.electron.ipcRenderer.invoke(
            'claim-subdomain',
            newCreatingApp.subdomain,
            newCreatingApp.appName
          );
        } catch (err) {
          console.error('Failed to claim subdomain:', err);
          // Don't fail the whole operation if subdomain claim fails
        }
      }

      toastSuccess({
        title: 'App created!',
        msg: `Successfully created ${newCreatingApp.displayName}${newCreatingApp.subdomain ? ` with subdomain ${newCreatingApp.subdomain}` : ''}`,
      });

      // Remove from creating apps and reload the apps list
      setCreatingApps(prev => prev.filter(app => app.id !== creatingAppId));
      await loadApps();
    } catch (err) {
      console.error('[BuildView] Failed to create app:', err);
      
      // Mark the creating app as having an error
      setCreatingApps(prev => prev.map(app => 
        app.id === creatingAppId 
          ? { 
              ...app, 
              hasError: true, 
              progress: err instanceof Error ? err.message : 'Unknown error occurred' 
            }
          : app
      ));

      toastError({
        title: 'Failed to create app',
        msg: err instanceof Error ? err.message : 'Unknown error occurred',
      });
    }
  };

  const handleCancelCreate = () => {
    setShowCreateDialog(false);
    setAppName('');
    setDisplayAppName('');
    setSubdomain('');
    setSubdomainAvailable(null);
    setSubdomainCheckError(null);
  };

  const handleAppClick = async (app: AppTile) => {
    try {
      // Get the secret key for authentication
      const secretKey = await window.electron.getSecretKey();

      // Setup the pairing session with the build extension
      const response = await fetch(getApiUrl('/extensions/setup-pairing'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify({
          project_dir: app.path
        })
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to setup pairing session');
      }

      const result = await response.json();

      // Check if there was an error in the response
      if (result.error) {
        throw new Error(result.message || 'Failed to setup pairing session');
      }

      // Get the port from the manage_server tool
      let port = 3000; // default
      try {
        const portResponse = await fetch(getApiUrl('/agent/call_tool'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': secretKey,
          },
          body: JSON.stringify({
            tool_name: 'build__manage_server',
            arguments: { action: 'port' }
          })
        });

        if (portResponse.ok) {
          const portResult = await portResponse.json();
          if (portResult.success && portResult.result && portResult.result.length > 0) {
            const portText = portResult.result[0].text;
            const parsedPort = parseInt(portText, 10);
            if (!isNaN(parsedPort)) {
              port = parsedPort;
            }
          }
        }
      } catch (err) {
        console.warn('Failed to get port from manage_server, using default 3000', err);
      }
      localStorage.setItem('goose-sidecar-url', `http://localhost:${port}`);

      // 1) Navigate to ChatView
      window.location.hash = '#/pair';

      // 2) Open sidecar browser immediately with the correct port
      // Use a small delay to let the router/layout mount
      setTimeout(() => {
        try {
          const evt = new CustomEvent('open-sidecar-localhost', { detail: { url: `http://localhost:${port}` } });
          window.dispatchEvent(evt);
        } catch (e) {
          console.warn('Failed to auto-open sidecar, user can click the globe button.', e);
        }
      }, 150);
    } catch (err) {
      console.error('Failed to open app:', err);
      toastError({
        title: 'Failed to open app',
        msg: err instanceof Error ? err.message : 'Could not open the app',
      });
    }
  };

  // Component for rendering individual app tiles (similar to SessionItem) - removed as not used

  return (
    <MainPanelLayout>
      <div className="bg-background-muted flex flex-col h-full">
        {/* Header container with rounded bottom - matching home page */}
        <div className="bg-background-default rounded-b-2xl mb-0.5">
          <div className="px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Build</h1>
                {/* Create App button on the right - show when not loading */}
                {!isLoading && (
                  <Button
                    onClick={handleCreateAppClick}
                    variant="default"
                    className="flex items-center gap-2"
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
        <div className="flex-1 overflow-hidden">
          <div className="h-full overflow-y-auto scroll-smooth" style={{ scrollSnapType: 'y mandatory' }}>
            <div className="flex flex-col space-y-0.5 p-0.5">
              {isLoading ? (
                /* Loading state - single container */
                <div className="bg-background-default rounded-2xl py-6 px-6 min-h-[400px]" style={{ scrollSnapAlign: 'start' }}>
                  <div className="flex items-center justify-center h-full text-text-muted">
                    <div className="flex items-center gap-2">
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-text-muted"></div>
                      <span>Loading apps...</span>
                    </div>
                  </div>
                </div>
              ) : (apps.length > 0 || creatingApps.length > 0) ? (
                /* Apps exist or creating - individual containers for each app */
                <>
                  {/* Render creating apps first (skeleton placeholders) */}
                  {creatingApps.map((creatingApp) => (
                <div key={creatingApp.id} className="bg-background-default rounded-2xl py-6 px-6 shadow-inner animate-pulse-slow" style={{ scrollSnapAlign: 'start' }}>
                  <div className="flex flex-col h-full text-text-muted page-transition">
                    <div className="flex flex-col items-start">
                      {/* App image placeholder - animated skeleton */}
                      <div className="w-8 h-8 bg-blue-100 rounded-md mb-3 flex items-center justify-center animate-pulse-slow">
                        <div className="w-4 h-4 bg-blue-200 rounded-sm"></div>
                      </div>

                      <h3 className="text-base truncate mb-1 text-text-default">{creatingApp.displayName}</h3>

                      {/* Subdomain if provided */}
                      {creatingApp.subdomain && (
                        <div className="flex items-center text-text-muted text-xs mb-1">
                          <Globe className="w-3 h-3 mr-1 flex-shrink-0" />
                          <span className="text-blue-600">
                            {creatingApp.subdomain}
                            {DOMAIN}
                          </span>
                        </div>
                      )}

                      {/* Creation progress */}
                      <div className="flex items-center text-text-muted text-xs mb-1">
                        <div className="animate-spin rounded-full h-3 w-3 border-b-2 border-text-subtle mr-1 flex-shrink-0"></div>
                        <span className={creatingApp.hasError ? 'text-red-600' : 'text-text-subtle'}>
                          {creatingApp.progress}
                        </span>
                      </div>

                      {/* Folder path placeholder */}
                      <div className="flex items-center text-text-muted text-xs mb-4">
                        <FolderOpen className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span className="truncate">~/goose/apps/{creatingApp.appName}</span>
                      </div>
                    </div>

                    <div className="flex justify-end mt-auto pt-6">
                      <Button
                        variant="secondary"
                        className="flex items-center gap-2 opacity-50"
                        disabled={true}
                      >
                        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-text-subtle"></div>
                        Creating...
                      </Button>
                    </div>
                  </div>
                </div>
              ))}

              {/* Render existing apps */}
              {apps.map((app) => (
                <div key={app.id} className="bg-background-default rounded-2xl py-6 px-6 shadow-lg transition-shadow duration-700 ease-out" style={{ scrollSnapAlign: 'start' }}>
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
                        <div
                          className={`w-4 h-4 ${appColors[app.id]?.inner || 'bg-background-strong'} rounded-sm`}
                        ></div>
                      </div>

                      <h3 className="text-base truncate mb-1 text-text-default">{app.app_name}</h3>

                      {/* Subdomain if claimed */}
                      {app.subdomain && (
                        <div className="flex items-center text-text-muted text-xs mb-1">
                          <Globe className="w-3 h-3 mr-1 flex-shrink-0" />
                          <span className="text-blue-600">
                            {app.subdomain}
                            {DOMAIN}
                          </span>
                        </div>
                      )}

                      {/* Timestamp */}
                      <div className="flex items-center text-text-muted text-xs mb-1">
                        <Clock className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span>{formatMessageTimestamp(app.last_edited)}</span>
                      </div>

                      {/* Path */}
                      <div 
                        className="flex items-center text-text-muted text-xs mb-4 cursor-pointer hover:text-text-standard transition-colors duration-200"
                        onClick={(e) => {
                          e.stopPropagation(); // Prevent triggering the app click
                          window.electron.openDirectoryInExplorer(app.path);
                        }}
                        title="Open in Finder"
                      >
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
                        <ChatSmart className="w-4 h-4" />
                        Open in chat
                      </Button>
                    </div>
                  </div>
                </div>
              ))}

                </>
              ) : (
                /* Empty state - two separate containers */
                <>
                  {/* Empty state content container */}
                  <div className="bg-background-default rounded-2xl py-6 px-6" style={{ scrollSnapAlign: 'start' }}>
                    <div className="flex flex-col h-full text-text-muted page-transition">
                      <div className="flex flex-col items-start">
                        <Hammer className="h-4 w-4 mb-4 text-inverse" />
                        <p className="text-sm">
                          Your web apps will show up here. Create a new web app to get started in build.
                        </p>
                      </div>
                      <div className="flex justify-end mt-auto pt-6">
                        <Button
                          onClick={handleCreateAppClick}
                          variant="default"
                          className="flex items-center gap-2"
                        >
                          <Plus className="w-4 h-4" />
                          Create your first web app
                        </Button>
                      </div>
                    </div>
                  </div>
                </>
              )}
            </div>
          </div>
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
                    setAppColors((prev) => ({
                      ...prev,
                      [colorPickerAppId]: newColors,
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
                  <div
                    className={`w-8 h-8 ${combo.bg} rounded-md mb-2 flex items-center justify-center group-hover:scale-110 transition-transform duration-200`}
                  >
                    <div className={`w-4 h-4 ${combo.inner} rounded-sm`}></div>
                  </div>
                  <span className="text-xs text-text-muted">{combo.name}</span>
                </div>
              ))}
            </div>

            <div className="flex justify-end">
              <Button onClick={() => setColorPickerAppId(null)} variant="ghost">
                Cancel
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Create App Dialog - Simple input form only */}
      {showCreateDialog && (
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[500px] max-w-[90vw]">
            <h3 className="text-lg font-medium text-text-standard mb-4">Create new web app</h3>

            <div className="flex flex-col h-[320px]">
              <div className="flex-1 space-y-4">
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
                    value={displayAppName}
                    onChange={(e) => {
                      const userInput = e.target.value;
                      setDisplayAppName(userInput);
                      const convertedName = convertToValidAppName(userInput);
                      setAppName(convertedName);
                    }}
                    placeholder="My Awesome App"
                    className="w-full"
                    autoFocus
                    onKeyDown={(e) => {
                      if (
                        e.key === 'Enter' &&
                        appName.trim() &&
                        (!subdomain.trim() || subdomainAvailable)
                      ) {
                        handleCreateApp();
                      } else if (e.key === 'Escape') {
                        handleCancelCreate();
                      }
                    }}
                  />
                  {displayAppName && appName !== displayAppName && (
                    <div className="mt-1 p-2 bg-background-muted rounded text-xs">
                      <span className="text-text-muted">App folder name: </span>
                      <span className="text-text-standard font-mono">{appName || '(invalid name)'}</span>
                    </div>
                  )}
                  <p className="text-xs text-text-muted mt-1">
                    Type any name you like - we'll automatically convert it to a valid folder name
                  </p>
                </div>

                {/* Subdomain field */}
                <div>
                  <label
                    htmlFor="subdomain"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Subdomain (Optional)
                  </label>
                  <div className="relative">
                    <Input
                      id="subdomain"
                      type="text"
                      value={subdomain}
                      onChange={(e) => {
                        const value = e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '');
                        setSubdomain(value);
                        setSubdomainAvailable(null);
                        setSubdomainCheckError(null);
                        // Debounce the check
                        if (value) {
                          setTimeout(() => {
                            if (value === e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '')) {
                              checkSubdomainAvailability(value);
                            }
                          }, 500);
                        }
                      }}
                      placeholder="my-site"
                      className={`w-full pr-10 ${!canConnectToSites ? 'opacity-50' : ''} ${
                        subdomain && subdomainAvailable === false
                          ? 'border-red-500'
                          : subdomain && subdomainAvailable === true
                            ? 'border-green-500'
                            : ''
                      }`}
                      disabled={!canConnectToSites}
                      onKeyDown={(e) => {
                        if (
                          e.key === 'Enter' &&
                          appName.trim() &&
                          (!subdomain.trim() || subdomainAvailable)
                        ) {
                          handleCreateApp();
                        } else if (e.key === 'Escape') {
                          handleCancelCreate();
                        }
                      }}
                    />
                    {/* Status indicator */}
                    {subdomain && (
                      <div className="absolute right-2 top-1/2 -translate-y-1/2">
                        {isCheckingSubdomain ? (
                          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500"></div>
                        ) : subdomainAvailable === true ? (
                          <div className="text-green-500">✓</div>
                        ) : subdomainAvailable === false ? (
                          <div className="text-red-500">✗</div>
                        ) : null}
                      </div>
                    )}
                  </div>
                  {!canConnectToSites ? (
                    <p className="text-xs text-yellow-600 mt-1">
                      Website claiming is currently unavailable (cannot connect to service)
                    </p>
                  ) : subdomain ? (
                    <p className="text-xs mt-1">
                      {isCheckingSubdomain ? (
                        <span className="text-text-muted">Checking availability...</span>
                      ) : subdomainAvailable === true ? (
                        <span className="text-green-600">
                          {subdomain}
                          {DOMAIN} is available!
                        </span>
                      ) : subdomainAvailable === false ? (
                        <span className="text-red-600">
                          {subdomain}
                          {DOMAIN} is already taken
                        </span>
                      ) : subdomainCheckError ? (
                        <span className="text-yellow-600">{subdomainCheckError}</span>
                      ) : (
                        <span className="text-text-muted">
                          Your site will be available at {subdomain}
                          {DOMAIN}
                        </span>
                      )}
                    </p>
                  ) : (
                    <p className="text-xs text-text-muted mt-1">
                      Claim a subdomain for your app (e.g., my-site{DOMAIN})
                    </p>
                  )}
                </div>
              </div>

              <div className="flex justify-end space-x-3 mt-6">
                <Button onClick={handleCancelCreate} variant="ghost">
                  Cancel
                </Button>
                <Button
                  onClick={handleCreateApp}
                  disabled={
                    !appName.trim() || (!!subdomain.trim() && !subdomainAvailable)
                  }
                  variant="default"
                  className="min-w-[120px]"
                >
                  Create App
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </MainPanelLayout>
  );
};

export default BuildView;
