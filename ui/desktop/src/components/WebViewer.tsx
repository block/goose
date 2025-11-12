import { useRef, useState, useEffect, useCallback } from 'react';
import { RefreshCw, ExternalLink, ChevronLeft, ChevronRight, Home, Globe, Shield, ShieldOff } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';

interface WebViewerProps {
  initialUrl?: string;
  onUrlChange?: (url: string) => void;
  allowAllSites?: boolean; // If false, restricts to localhost only
}

function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

function isLocalhostUrl(url: string): boolean {
  try {
    const parsedUrl = new URL(url);
    return (
      parsedUrl.protocol === 'http:' &&
      (parsedUrl.hostname === 'localhost' || parsedUrl.hostname === '127.0.0.1')
    );
  } catch {
    return false;
  }
}

function formatUrl(input: string, allowAllSites: boolean = true): string {
  const trimmed = input.trim();
  
  // If it's just a port number, prepend localhost
  if (/^\d+$/.test(trimmed)) {
    return `http://localhost:${trimmed}`;
  }

  // If it starts with localhost: or 127.0.0.1: without protocol
  if (/^(localhost|127\.0\.0\.1):\d+/.test(trimmed)) {
    return `http://${trimmed}`;
  }

  // If it's already a complete URL, return as-is
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    return trimmed;
  }

  if (allowAllSites) {
    // Check if it looks like a search query (contains spaces or no dots)
    if (trimmed.includes(' ') || (!trimmed.includes('.') && !trimmed.includes('localhost'))) {
      // Convert to Google search
      return `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`;
    }
    
    // Check if it looks like a domain
    if (trimmed.includes('.')) {
      return `https://${trimmed}`;
    }
    
    // If it's a single word, try as a .com domain first
    return `https://${trimmed}.com`;
  }

  return trimmed;
}

function getDomainFromUrl(url: string): string {
  try {
    const parsedUrl = new URL(url);
    return parsedUrl.hostname;
  } catch {
    return url;
  }
}

export function WebViewer({
  initialUrl = 'http://localhost:3000',
  onUrlChange,
  allowAllSites = true,
}: WebViewerProps) {
  // Generate a truly unique ID for this BrowserView instance
  const browserViewId = useRef(`webviewer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}-${performance.now().toString(36)}`);
  const containerRef = useRef<HTMLDivElement>(null);
  const updateBoundsTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // Initialize from localStorage or use initialUrl
  const storageKey = allowAllSites ? 'goose-webviewer-url' : 'goose-sidecar-url';
  const [url, setUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(storageKey) || initialUrl;
    }
    return initialUrl;
  });

  const [inputUrl, setInputUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(storageKey) || initialUrl;
    }
    return initialUrl;
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canGoBack, setCanGoBack] = useState(false);
  const [canGoForward, setCanGoForward] = useState(false);
  const [isSecure, setIsSecure] = useState(false);
  const [actualUrl, setActualUrl] = useState(url);
  const [pageTitle, setPageTitle] = useState('');
  const [browserViewCreated, setBrowserViewCreated] = useState(false);

  // Create BrowserView on mount with timeout and retry logic
  useEffect(() => {
    const createBrowserView = async (retryCount = 0) => {
      if (!containerRef.current) return;

      try {
        console.log(`[WebViewer-${browserViewId.current}] Attempting to create BrowserView with URL:`, url, `(attempt ${retryCount + 1})`);
        
        const rect = containerRef.current.getBoundingClientRect();
        console.log(`[WebViewer-${browserViewId.current}] Container bounds:`, rect);
        
        // BrowserView expects coordinates relative to the window content area
        // getBoundingClientRect() returns viewport coordinates, so we need to adjust
        const adjustedBounds = {
          x: Math.round(rect.x),
          y: Math.round(rect.y), 
          width: Math.round(Math.max(rect.width, 100)), // Minimum width
          height: Math.round(Math.max(rect.height, 100)), // Minimum height
        };
        
        console.log(`[WebViewer-${browserViewId.current}] Adjusted BrowserView bounds:`, adjustedBounds);
        
        // Add timeout to browser view creation
        const createWithTimeout = Promise.race([
          window.electron.createBrowserView(url, adjustedBounds),
          new Promise((_, reject) => 
            setTimeout(() => reject(new Error('Browser view creation timeout')), 10000)
          )
        ]);
        
        const result = await createWithTimeout as any;
        
        console.log(`[WebViewer-${browserViewId.current}] BrowserView creation result:`, result);
        
        if (result.success && result.viewId) {
          browserViewId.current = result.viewId;
          setBrowserViewCreated(true);
          console.log(`[WebViewer-${browserViewId.current}] BrowserView created successfully:`, result.viewId);
        } else {
          throw new Error(result.error || 'Unknown error');
        }
      } catch (err) {
        console.error(`[WebViewer-${browserViewId.current}] Error creating BrowserView:`, err);
        
        // Retry logic for transient failures
        if (retryCount < 2 && (err.message?.includes('timeout') || err.message?.includes('ECONNREFUSED'))) {
          console.log(`[WebViewer-${browserViewId.current}] Retrying browser view creation in 1 second...`);
          setTimeout(() => createBrowserView(retryCount + 1), 1000);
          return;
        }
        
        setError(`Failed to initialize browser view: ${err.message || 'Unknown error'}`);
      }
    };

    // Add a small delay to ensure the container is properly rendered
    const timer = setTimeout(() => {
      if (containerRef.current) {
        createBrowserView();
      }
    }, 100);

    return () => {
      clearTimeout(timer);
    };
  }, [url]);

  // Cleanup BrowserView on component unmount
  useEffect(() => {
    return () => {
      console.log('WebViewer component unmounting, cleaning up BrowserView:', browserViewId.current);
      if (browserViewId.current) {
        window.electron.destroyBrowserView(browserViewId.current).catch((err) => {
          console.error('Error destroying BrowserView on unmount:', err);
        });
      }
    };
  }, []);

  // Update BrowserView bounds when container resizes (throttled)
  useEffect(() => {
    if (!browserViewCreated || !containerRef.current) return;

    const throttledUpdateBounds = () => {
      // Clear any pending update
      if (updateBoundsTimeoutRef.current) {
        clearTimeout(updateBoundsTimeoutRef.current);
      }
      
      // Schedule new update (throttled to max 60fps)
      updateBoundsTimeoutRef.current = setTimeout(() => {
        if (containerRef.current && browserViewId.current) {
          const rect = containerRef.current.getBoundingClientRect();
          
          // Use exact container bounds to prevent clipping
          const adjustedBounds = {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(Math.max(rect.width, 100)), // Minimum width
            height: Math.round(Math.max(rect.height, 100)), // Minimum height
          };
          
          // Validate bounds are positive and reasonable
          if (adjustedBounds.width > 0 && adjustedBounds.height > 0 && 
              adjustedBounds.x >= 0 && adjustedBounds.y >= 0) {
            console.log(`[WebViewer-${browserViewId.current}] Updating BrowserView bounds:`, adjustedBounds);
            window.electron.updateBrowserViewBounds(browserViewId.current, adjustedBounds).catch(console.error);
          } else {
            console.warn(`[WebViewer-${browserViewId.current}] Invalid bounds calculated:`, adjustedBounds);
          }
        }
      }, 16); // ~60fps throttling
    };

    // Initial bounds update
    throttledUpdateBounds();

    // Set up resize observer for the container
    const resizeObserver = new ResizeObserver(throttledUpdateBounds);
    resizeObserver.observe(containerRef.current);

    // Listen for window resize and scroll events
    window.addEventListener('resize', throttledUpdateBounds);
    window.addEventListener('scroll', throttledUpdateBounds);
    
    // Use MutationObserver with broader scope to catch drag operations
    const mutationObserver = new MutationObserver((mutations) => {
      // Check if any mutations affect positioning
      let shouldUpdate = false;
      for (const mutation of mutations) {
        // Check for style changes that might affect positioning
        if (mutation.type === 'attributes' && mutation.attributeName === 'style') {
          shouldUpdate = true;
          break;
        }
        // Check for DOM structure changes that might affect layout
        if (mutation.type === 'childList') {
          shouldUpdate = true;
          break;
        }
      }
      if (shouldUpdate) {
        throttledUpdateBounds();
      }
    });
    
    // Observe the container and walk up the DOM tree to catch drag operations
    mutationObserver.observe(containerRef.current, {
      attributes: true,
      attributeFilter: ['style', 'class']
    });
    
    // Also observe the parent container (walk up to find the bento container)
    let currentParent = containerRef.current.parentElement;
    while (currentParent) {
      if (currentParent.classList.contains('relative') || 
          currentParent.classList.contains('flex-1') ||
          currentParent.getAttribute('data-container-type')) {
        resizeObserver.observe(currentParent);
        break;
      }
      currentParent = currentParent.parentElement;
    }
    
    // Observe parent elements up to the bento box to catch drag/layout changes
    let ancestorElement = containerRef.current.parentElement;
    let depth = 0;
    const maxDepth = 5; // Limit depth to avoid performance issues
    
    while (ancestorElement && depth < maxDepth) {
      mutationObserver.observe(ancestorElement, {
        childList: true,
        attributes: true,
        attributeFilter: ['style', 'class', 'transform']
      });
      
      // Stop if we reach the bento container or main layout
      if (ancestorElement.classList.contains('bento') || 
          ancestorElement.classList.contains('enhanced-bento') ||
          ancestorElement.getAttribute('data-container-type')) {
        break;
      }
      
      ancestorElement = ancestorElement.parentElement;
      depth++;
    }
    
    // Also use an interval as a fallback to ensure bounds stay in sync
    const boundsCheckInterval = setInterval(() => {
      if (containerRef.current && browserViewId.current) {
        const rect = containerRef.current.getBoundingClientRect();
        // Only update if position has actually changed
        const currentBounds = {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(Math.max(rect.height, 100)),
        };
        
        // Store last bounds to detect changes
        if (!containerRef.current.dataset.lastBounds) {
          containerRef.current.dataset.lastBounds = JSON.stringify(currentBounds);
          return;
        }
        
        const lastBounds = JSON.parse(containerRef.current.dataset.lastBounds);
        if (currentBounds.x !== lastBounds.x || 
            currentBounds.y !== lastBounds.y ||
            currentBounds.width !== lastBounds.width ||
            currentBounds.height !== lastBounds.height) {
          
          console.log(`[WebViewer-${browserViewId.current}] Position changed, updating bounds:`, currentBounds);
          containerRef.current.dataset.lastBounds = JSON.stringify(currentBounds);
          window.electron.updateBrowserViewBounds(browserViewId.current, currentBounds).catch(console.error);
        }
      }
    }, 100); // Check every 100ms

    return () => {
      // Clear any pending bounds update
      if (updateBoundsTimeoutRef.current) {
        clearTimeout(updateBoundsTimeoutRef.current);
        updateBoundsTimeoutRef.current = null;
      }
      
      // Clear the bounds check interval
      clearInterval(boundsCheckInterval);
      
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      window.removeEventListener('resize', throttledUpdateBounds);
      window.removeEventListener('scroll', throttledUpdateBounds);
    };
  }, [browserViewCreated]);

  // Navigate to initial URL when BrowserView is ready
  useEffect(() => {
    if (browserViewCreated && url) {
      navigateToUrl(url);
    }
  }, [browserViewCreated]);

  // Update navigation state periodically
  useEffect(() => {
    if (!browserViewCreated) return;

    const updateNavigationState = async () => {
      try {
        const state = await window.electron.getBrowserViewNavigationState(browserViewId.current);
        if (state) {
          setCanGoBack(state.canGoBack);
          setCanGoForward(state.canGoForward);
          
          if (state.url && state.url !== actualUrl) {
            setActualUrl(state.url);
            setInputUrl(state.url);
            setIsSecure(state.url.startsWith('https://'));
          }
          
          if (state.title) {
            setPageTitle(state.title);
          }
        }
      } catch (err) {
        console.error('Error getting navigation state:', err);
      }
    };

    // Update immediately and then periodically
    updateNavigationState();
    const interval = setInterval(updateNavigationState, 1000);

    return () => clearInterval(interval);
  }, [browserViewCreated, actualUrl]);



  const navigateToUrl = async (targetUrl: string) => {
    if (!browserViewCreated) return;

    setIsLoading(true);
    setError(null);

    try {
      const success = await window.electron.browserViewNavigate(browserViewId.current, targetUrl);
      if (!success) {
        setError('Failed to navigate to URL');
        setIsLoading(false);
      }
      // Loading state will be cleared by navigation state updates
    } catch (err) {
      console.error('Navigation error:', err);
      setError('Navigation failed');
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (onUrlChange) {
      onUrlChange(actualUrl);
    }
  }, [actualUrl, onUrlChange]);

  const handleUrlSubmit = (newUrl: string) => {
    const formattedUrl = formatUrl(newUrl, allowAllSites);

    if (!isValidUrl(formattedUrl)) {
      setError('Please enter a valid URL');
      return;
    }

    if (!allowAllSites && !isLocalhostUrl(formattedUrl)) {
      setError('Only localhost URLs are allowed (e.g., http://localhost:3000)');
      return;
    }

    setError(null);
    setUrl(formattedUrl);
    setInputUrl(formattedUrl);

    // Save to localStorage
    if (typeof window !== 'undefined') {
      localStorage.setItem(storageKey, formattedUrl);
    }

    navigateToUrl(formattedUrl);
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleUrlSubmit(inputUrl);
    }
  };

  const handleRefresh = async () => {
    if (!browserViewCreated) return;
    
    try {
      await window.electron.browserViewRefresh(browserViewId.current);
    } catch (err) {
      console.error('Refresh error:', err);
    }
  };

  const handleOpenInBrowser = () => {
    window.electron.openExternal(actualUrl).catch(console.error);
  };

  const handleGoBack = async () => {
    if (!browserViewCreated || !canGoBack) return;
    
    try {
      await window.electron.browserViewGoBack(browserViewId.current);
    } catch (err) {
      console.error('Go back error:', err);
    }
  };

  const handleGoForward = async () => {
    if (!browserViewCreated || !canGoForward) return;
    
    try {
      await window.electron.browserViewGoForward(browserViewId.current);
    } catch (err) {
      console.error('Go forward error:', err);
    }
  };

  const handleHome = () => {
    const homeUrl = allowAllSites ? 'https://google.com' : 'http://localhost:3000';
    handleUrlSubmit(homeUrl);
  };

  const domain = getDomainFromUrl(actualUrl);
  const isLocalhost = isLocalhostUrl(actualUrl);

  return (
    <div className="h-full flex flex-col bg-background-default rounded-lg">
      {/* URL Bar and Controls */}
      <div className="flex items-center gap-2 p-3 bg-background-muted rounded-t-lg">
        {/* Navigation buttons */}
        <div className="flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleGoBack}
                disabled={!canGoBack}
                className="p-1 h-8 w-8"
              >
                <ChevronLeft size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Go Back</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleGoForward}
                disabled={!canGoForward}
                className="p-1 h-8 w-8"
              >
                <ChevronRight size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Go Forward</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleRefresh} className="p-1 h-8 w-8">
                <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Refresh</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleHome} className="p-1 h-8 w-8">
                <Home size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{allowAllSites ? 'Go to Google' : 'Go to localhost:3000'}</TooltipContent>
          </Tooltip>
        </div>

        {/* URL Input with security indicator */}
        <div className="flex-1 flex items-center">
          <div className="flex-1 relative flex items-center">
            {/* Security/Type indicator */}
            <div className="absolute left-2 z-10 flex items-center">
              {isLocalhost ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <Globe size={12} className="text-blue-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Local development server</TooltipContent>
                </Tooltip>
              ) : isSecure ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <Shield size={12} className="text-green-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Secure connection (HTTPS)</TooltipContent>
                </Tooltip>
              ) : (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <ShieldOff size={12} className="text-orange-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Insecure connection (HTTP)</TooltipContent>
                </Tooltip>
              )}
            </div>
            
            <input
              type="text"
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              onKeyPress={handleKeyPress}
              placeholder={allowAllSites ? "Enter URL or search term" : "http://localhost:3000"}
              className="w-full pl-8 pr-3 py-1 text-sm border border-border-subtle rounded-md bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-border-prominent focus:border-transparent"
            />
          </div>
          
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleUrlSubmit(inputUrl)}
            className="ml-2 px-3 py-1 text-xs"
          >
            Go
          </Button>
        </div>

        {/* External link button */}
        <div className="flex items-center gap-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleOpenInBrowser} className="p-1 h-8 w-8">
                <ExternalLink size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Open in Browser</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="p-3 bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800">
          <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
        </div>
      )}

      {/* BrowserView Container */}
      <div 
        ref={containerRef}
        className="flex-1 relative overflow-hidden bg-background-default rounded-b-lg"
        style={{ 
          isolation: 'isolate', // Create new stacking context
          zIndex: 1 // Ensure it's below UI controls
        }}
      >
        {!browserViewCreated && (
          <div className="absolute inset-0 bg-background-default flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-text-subtle text-sm">Initializing browser...</p>
            </div>
          </div>
        )}
        
        {browserViewCreated && isLoading && (
          <div className="absolute top-0 left-0 right-0 h-1 bg-blue-200">
            <div className="h-full bg-blue-500 animate-pulse"></div>
          </div>
        )}
        
        {pageTitle && (
          <div className="absolute top-2 left-2 bg-black/70 text-white text-xs px-2 py-1 rounded opacity-0 hover:opacity-100 transition-opacity pointer-events-none">
            {pageTitle}
          </div>
        )}
      </div>
    </div>
  );
}

export default WebViewer;
