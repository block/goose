import React, { useState, useEffect, useRef, useCallback } from 'react';
import { ArrowLeft, ArrowRight, RotateCcw, Home, Globe, X } from 'lucide-react';
import { Button } from './ui/button';

interface WebBrowserProps {
  initialUrl?: string;
  title?: string;
  onClose?: () => void;
  className?: string;
}

interface NavigationState {
  canGoBack: boolean;
  canGoForward: boolean;
  isLoading: boolean;
  url: string;
}

export const WebBrowser: React.FC<WebBrowserProps> = ({
  initialUrl = 'https://google.com',
  title = 'Web Browser',
  onClose,
  className = ''
}) => {
  const [currentUrl, setCurrentUrl] = useState(initialUrl);
  const [inputUrl, setInputUrl] = useState(initialUrl);
  const [viewId, setViewId] = useState<string | null>(null);
  const [navigationState, setNavigationState] = useState<NavigationState>({
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    url: initialUrl
  });
  const [isInitialized, setIsInitialized] = useState(false);
  const [isEditingUrl, setIsEditingUrl] = useState(false);
  const browserContainerRef = useRef<HTMLDivElement>(null);
  const updateTimeoutRef = useRef<NodeJS.Timeout>();
  const instanceIdRef = useRef<string>(`browser-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`);
  const isVisibleRef = useRef<boolean>(true);
  const isEditingUrlRef = useRef<boolean>(false);

  // Create BrowserView when component mounts
  useEffect(() => {
    const initializeBrowser = async () => {
      if (!browserContainerRef.current || isInitialized) return;

      const container = browserContainerRef.current;
      const rect = container.getBoundingClientRect();
      
      // Use the container bounds directly (it's already positioned below the address bar)
      const bounds = {
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height
      };

      try {
        const result = await window.electron.createBrowserView(initialUrl, bounds);
        
        if (result.success && result.viewId) {
          setViewId(result.viewId);
          setIsInitialized(true);
          console.log(`WebBrowser [${instanceIdRef.current}]: BrowserView created successfully:`, result.viewId);
          
          // Start polling for navigation state
          pollNavigationState(result.viewId);
        } else {
          console.error(`WebBrowser [${instanceIdRef.current}]: Failed to create BrowserView:`, result.error);
        }
      } catch (error) {
        console.error(`WebBrowser [${instanceIdRef.current}]: Error creating BrowserView:`, error);
      }
    };

    initializeBrowser();

    // Cleanup on unmount
    return () => {
      console.log(`WebBrowser [${instanceIdRef.current}]: Cleaning up, destroying BrowserView:`, viewId);
      if (viewId) {
        window.electron.destroyBrowserView(viewId).catch((error) => {
          console.error(`WebBrowser [${instanceIdRef.current}]: Error destroying BrowserView:`, error);
        });
      }
      if (updateTimeoutRef.current) {
        clearTimeout(updateTimeoutRef.current);
      }
    };
  }, [initialUrl, isInitialized]); // Removed pollNavigationState dependency

  // Listen for sidecar closing events to trigger explicit cleanup
  useEffect(() => {
    const handleSidecarClosing = (event: CustomEvent) => {
      const { tabId, viewId: closingViewId } = event.detail;
      console.log(`WebBrowser [${instanceIdRef.current}]: Received sidecar closing event for tabId:${tabId}, viewId:${closingViewId}`);
      
      // If this WebBrowser instance should be cleaned up, destroy the BrowserView immediately
      if (viewId) {
        console.log(`WebBrowser [${instanceIdRef.current}]: Triggering immediate cleanup due to sidecar closing`);
        window.electron.destroyBrowserView(viewId).catch((error) => {
          console.error(`WebBrowser [${instanceIdRef.current}]: Error destroying BrowserView during sidecar close:`, error);
        });
        setViewId(null);
        setIsInitialized(false);
      }
    };

    window.addEventListener('sidecar-web-view-closing', handleSidecarClosing as EventListener);
    
    return () => {
      window.removeEventListener('sidecar-web-view-closing', handleSidecarClosing as EventListener);
    };
  }, [viewId]);

  // Sync ref with state
  useEffect(() => {
    isEditingUrlRef.current = isEditingUrl;
  }, [isEditingUrl]);

  // Poll navigation state
  const pollNavigationState = useCallback((browserViewId: string) => {
    const updateState = async () => {
      try {
        const state = await window.electron.getBrowserViewNavigationState(browserViewId);
        if (state) {
          setNavigationState(state);
          setCurrentUrl(state.url);
          
          // Only update input URL if user is not actively editing it
          if (!isEditingUrlRef.current) {
            setInputUrl(state.url);
          }
        }
      } catch (error) {
        console.error('WebBrowser: Error getting navigation state:', error);
      }
      
      // Continue polling
      updateTimeoutRef.current = setTimeout(() => updateState(), 1000);
    };

    updateState();
  }, []); // No dependencies to prevent recreation

  // Manage BrowserView visibility based on component visibility
  useEffect(() => {
    if (!viewId) return;

    // Show the BrowserView when component mounts or becomes visible
    const showBrowserView = async () => {
      if (!browserContainerRef.current) return;
      
      const container = browserContainerRef.current;
      const rect = container.getBoundingClientRect();
      
      // Only show if the container is actually visible (has dimensions)
      if (rect.width > 0 && rect.height > 0) {
        const bounds = {
          x: rect.left,
          y: rect.top, // Use the container's actual top position
          width: rect.width,
          height: rect.height // Use the container's actual height
        };
        
        await window.electron.updateBrowserViewBounds(viewId, bounds);
        isVisibleRef.current = true;
        console.log(`WebBrowser [${instanceIdRef.current}]: BrowserView shown with bounds:`, bounds);
      }
    };

    showBrowserView();

    // Hide the BrowserView when component unmounts
    return () => {
      if (viewId && isVisibleRef.current) {
        // Hide by setting bounds to zero
        window.electron.updateBrowserViewBounds(viewId, { x: 0, y: 0, width: 0, height: 0 }).catch(console.error);
        isVisibleRef.current = false;
        console.log(`WebBrowser [${instanceIdRef.current}]: BrowserView hidden on unmount`);
      }
    };
  }, [viewId]);

  // Update BrowserView bounds when container resizes
  useEffect(() => {
    const updateBounds = () => {
      if (!viewId || !browserContainerRef.current || !isVisibleRef.current) return;

      const container = browserContainerRef.current;
      const rect = container.getBoundingClientRect();
      
      // Only update if the container has dimensions (is visible)
      if (rect.width > 0 && rect.height > 0) {
        const bounds = {
          x: rect.left,
          y: rect.top, // Use container's actual position
          width: rect.width,
          height: rect.height
        };

        window.electron.updateBrowserViewBounds(viewId, bounds).catch(console.error);
      }
    };

    const resizeObserver = new ResizeObserver(updateBounds);
    if (browserContainerRef.current) {
      resizeObserver.observe(browserContainerRef.current);
    }

    return () => {
      resizeObserver.disconnect();
    };
  }, [viewId]);

  // Navigation handlers
  const handleNavigate = useCallback(async (url: string) => {
    if (!viewId) return;

    try {
      // Ensure URL has protocol
      let fullUrl = url;
      if (!url.startsWith('http://') && !url.startsWith('https://')) {
        // Check if it looks like a domain
        if (url.includes('.') && !url.includes(' ')) {
          fullUrl = `https://${url}`;
        } else {
          // Treat as search query
          fullUrl = `https://www.google.com/search?q=${encodeURIComponent(url)}`;
        }
      }

      await window.electron.browserViewNavigate(viewId, fullUrl);
      setCurrentUrl(fullUrl);
      setInputUrl(fullUrl);
    } catch (error) {
      console.error('WebBrowser: Navigation error:', error);
    }
  }, [viewId]);

  const handleGoBack = useCallback(async () => {
    if (!viewId || !navigationState.canGoBack) return;
    
    try {
      await window.electron.browserViewGoBack(viewId);
    } catch (error) {
      console.error('WebBrowser: Go back error:', error);
    }
  }, [viewId, navigationState.canGoBack]);

  const handleGoForward = useCallback(async () => {
    if (!viewId || !navigationState.canGoForward) return;
    
    try {
      await window.electron.browserViewGoForward(viewId);
    } catch (error) {
      console.error('WebBrowser: Go forward error:', error);
    }
  }, [viewId, navigationState.canGoForward]);

  const handleRefresh = useCallback(async () => {
    if (!viewId) return;
    
    try {
      await window.electron.browserViewRefresh(viewId);
    } catch (error) {
      console.error('WebBrowser: Refresh error:', error);
    }
  }, [viewId]);

  const handleHome = useCallback(() => {
    handleNavigate('https://google.com');
  }, [handleNavigate]);

  const handleUrlSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    setIsEditingUrl(false);
    handleNavigate(inputUrl);
  }, [inputUrl, handleNavigate]);

  const handleUrlChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setIsEditingUrl(true);
    setInputUrl(e.target.value);
  }, []);

  const handleUrlFocus = useCallback(() => {
    setIsEditingUrl(true);
  }, []);

  const handleUrlBlur = useCallback(() => {
    // Delay setting editing to false to allow for form submission
    setTimeout(() => {
      setIsEditingUrl(false);
    }, 100);
  }, []);

  return (
    <div className={`flex flex-col h-full bg-background-default ${className}`}>
      {/* Browser Toolbar */}
      <div className="flex items-center gap-2 p-3 border-b border-border-subtle bg-background-muted">
        {/* Navigation Buttons */}
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGoBack}
            disabled={!navigationState.canGoBack}
            title="Go back"
          >
            <ArrowLeft size={16} />
          </Button>
          
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGoForward}
            disabled={!navigationState.canGoForward}
            title="Go forward"
          >
            <ArrowRight size={16} />
          </Button>
          
          <Button
            variant="ghost"
            size="sm"
            onClick={handleRefresh}
            disabled={navigationState.isLoading}
            title="Refresh"
          >
            <RotateCcw size={16} className={navigationState.isLoading ? 'animate-spin' : ''} />
          </Button>
          
          <Button
            variant="ghost"
            size="sm"
            onClick={handleHome}
            title="Home"
          >
            <Home size={16} />
          </Button>
        </div>

        {/* Address Bar */}
        <form onSubmit={handleUrlSubmit} className="flex-1 flex items-center gap-2">
          <div className="flex-1 relative">
            <Globe size={16} className="absolute left-3 top-1/2 transform -translate-y-1/2 text-text-muted" />
            <input
              type="text"
              value={inputUrl}
              onChange={handleUrlChange}
              onFocus={handleUrlFocus}
              onBlur={handleUrlBlur}
              placeholder="Enter URL or search..."
              className="w-full pl-10 pr-4 py-2 text-sm bg-background-default border border-border-subtle rounded-lg focus:outline-none focus:ring-2 focus:ring-border-prominent focus:border-transparent"
            />
          </div>
        </form>

        {/* Close Button */}
        {onClose && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            title="Close browser"
          >
            <X size={16} />
          </Button>
        )}
      </div>

      {/* Browser Content Area */}
      <div 
        ref={browserContainerRef}
        className="flex-1 relative overflow-hidden"
        style={{ 
          background: 'transparent',
          margin: 0,
          padding: 0,
          border: 'none'
        }}
      >
        {!isInitialized && (
          <div className="absolute inset-0 flex items-center justify-center bg-background-muted">
            <div className="flex items-center gap-2 text-text-muted">
              <RotateCcw size={20} className="animate-spin" />
              <span>Loading browser...</span>
            </div>
          </div>
        )}
        
        {/* The actual web content will be rendered by Electron's BrowserView */}
        {/* This div serves as a placeholder and positioning reference */}
      </div>


    </div>
  );
};

export default WebBrowser;
