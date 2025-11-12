import { useRef, useState, useEffect, useCallback } from 'react';
import { RefreshCw, ExternalLink, ChevronLeft, ChevronRight, Home, Globe, Shield, ShieldOff } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { useWebViewerContextOptional } from '../contexts/WebViewerContext';
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';

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
  // Generate a unique ID for this child window instance
  const childWindowId = useRef(`webviewer-window-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`);
  const containerRef = useRef<HTMLDivElement>(null);
  const updateBoundsTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // WebViewer context for AI prompt injection
  const webViewerContext = useWebViewerContextOptional();
  
  // Unified sidecar context for comprehensive AI context
  const unifiedSidecarContext = useUnifiedSidecarContextOptional();
  
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
  const [childWindowCreated, setChildWindowCreated] = useState(false);
  const [isVisible, setIsVisible] = useState(true);

  // Create child window on mount with timeout and retry logic
  useEffect(() => {
    const createChildWindow = async (retryCount = 0) => {
      if (!containerRef.current) return;

      try {
        console.log(`[WebViewer-${childWindowId.current}] Attempting to create child window with URL:`, url, `(attempt ${retryCount + 1})`);
        
        const rect = containerRef.current.getBoundingClientRect();
        console.log(`[WebViewer-${childWindowId.current}] Container bounds:`, rect);
        
        // Child window expects coordinates relative to the main window
        // getBoundingClientRect() returns viewport coordinates, so we use them directly
        const adjustedBounds = {
          x: Math.round(rect.x),
          y: Math.round(rect.y), 
          width: Math.round(Math.max(rect.width, 300)), // Minimum width for child window
          height: Math.round(Math.max(rect.height, 200)), // Minimum height for child window
        };
        
        console.log(`[WebViewer-${childWindowId.current}] Adjusted child window bounds:`, adjustedBounds);
        
        // Add timeout to child window creation
        const createWithTimeout = Promise.race([
          window.electron.createChildWebViewer(url, adjustedBounds, childWindowId.current),
          new Promise((_, reject) => 
            setTimeout(() => reject(new Error('Child window creation timeout')), 10000)
          )
        ]);
        
        const result = await createWithTimeout as any;
        
        console.log(`[WebViewer-${childWindowId.current}] Child window creation result:`, result);
        
        if (result.success && result.viewerId) {
          childWindowId.current = result.viewerId;
          setChildWindowCreated(true);
          console.log(`[WebViewer-${childWindowId.current}] Child window created successfully:`, result.viewerId);
          
          // Show the child window if we're visible
          if (isVisible) {
            await window.electron.showChildWebViewer(childWindowId.current);
          }
        } else {
          throw new Error(result.error || 'Unknown error');
        }
      } catch (err) {
        console.error(`[WebViewer-${childWindowId.current}] Error creating child window:`, err);
        
        // Retry logic for transient failures
        if (retryCount < 2 && (err.message?.includes('timeout') || err.message?.includes('ECONNREFUSED'))) {
          console.log(`[WebViewer-${childWindowId.current}] Retrying child window creation in 1 second...`);
          setTimeout(() => createChildWindow(retryCount + 1), 1000);
          return;
        }
        
        setError(`Failed to initialize child window: ${err.message || 'Unknown error'}`);
      }
    };

    // Add a small delay to ensure the container is properly rendered
    const timer = setTimeout(() => {
      if (containerRef.current) {
        createChildWindow();
      }
    }, 100);

    return () => {
      clearTimeout(timer);
    };
  }, [url, isVisible]);

  // Register/unregister with WebViewer context for AI prompt injection
  useEffect(() => {
    if (!webViewerContext || !childWindowCreated) return;
    
    const webViewerInfo = {
      id: childWindowId.current,
      url: actualUrl || url,
      title: pageTitle || 'Loading...',
      domain: getDomainFromUrl(actualUrl || url),
      isSecure: isSecure,
      isLocalhost: isLocalhostUrl(actualUrl || url),
      lastUpdated: new Date(),
      type: (allowAllSites ? 'main' : 'sidecar') as 'sidecar' | 'main',
    };

    webViewerContext.registerWebViewer(webViewerInfo);
    console.log('[WebViewer] Registered with context:', webViewerInfo);

    return () => {
      webViewerContext.unregisterWebViewer(childWindowId.current);
      console.log('[WebViewer] Unregistered from context:', childWindowId.current);
    };
  }, [webViewerContext, childWindowCreated, allowAllSites]);

  // Update WebViewer context when URL or title changes (debounced)
  useEffect(() => {
    if (!webViewerContext || !childWindowCreated) return;

    const updateTimeout = setTimeout(() => {
      webViewerContext.updateWebViewer(childWindowId.current, {
        url: actualUrl || url,
        title: pageTitle || 'Loading...',
        domain: getDomainFromUrl(actualUrl || url),
        isSecure: isSecure,
        isLocalhost: isLocalhostUrl(actualUrl || url),
      });
    }, 100); // Debounce updates to prevent rapid re-renders

    return () => clearTimeout(updateTimeout);
  }, [webViewerContext, childWindowCreated, actualUrl, pageTitle, isSecure, url]);

  // Register/unregister with Unified Sidecar context for comprehensive AI context
  useEffect(() => {
    if (!unifiedSidecarContext || !childWindowCreated) return;
    
    const sidecarInfo = {
      id: childWindowId.current,
      type: 'web-viewer' as const,
      title: pageTitle || 'Web Browser',
      url: actualUrl || url,
      domain: getDomainFromUrl(actualUrl || url),
      isSecure: isSecure,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
      isLoading: isLoading,
      timestamp: Date.now(),
    };

    unifiedSidecarContext.registerSidecar(sidecarInfo);
    console.log('[WebViewer] Registered with unified sidecar context:', sidecarInfo);

    return () => {
      unifiedSidecarContext.unregisterSidecar(childWindowId.current);
      console.log('[WebViewer] Unregistered from unified sidecar context:', childWindowId.current);
    };
  }, [unifiedSidecarContext, childWindowCreated]);

  // Update Unified Sidecar context when state changes (debounced)
  useEffect(() => {
    if (!unifiedSidecarContext || !childWindowCreated) return;

    const updateTimeout = setTimeout(() => {
      unifiedSidecarContext.updateSidecar(childWindowId.current, {
        title: pageTitle || 'Web Browser',
        url: actualUrl || url,
        domain: getDomainFromUrl(actualUrl || url),
        isSecure: isSecure,
        canGoBack: canGoBack,
        canGoForward: canGoForward,
        isLoading: isLoading,
        timestamp: Date.now(),
      });
    }, 100); // Debounce updates to prevent rapid re-renders

    return () => clearTimeout(updateTimeout);
  }, [unifiedSidecarContext, childWindowCreated, actualUrl, pageTitle, isSecure, canGoBack, canGoForward, isLoading]);

  // Cleanup child window on component unmount with robust error handling
  useEffect(() => {
    const cleanup = async () => {
      console.log('WebViewer component unmounting, cleaning up child window:', childWindowId.current);
      
      // Unregister from contexts first
      if (webViewerContext) {
        webViewerContext.unregisterWebViewer(childWindowId.current);
      }
      if (unifiedSidecarContext) {
        unifiedSidecarContext.unregisterSidecar(childWindowId.current);
      }
      
      if (childWindowId.current && childWindowCreated) {
        try {
          // First hide the child window
          await window.electron.hideChildWebViewer(childWindowId.current);
          // Then destroy it
          await window.electron.destroyChildWebViewer(childWindowId.current);
          console.log('Child window cleanup completed successfully');
        } catch (err) {
          console.error('Error destroying child window on unmount:', err);
          // Force cleanup even if there's an error
          try {
            await window.electron.destroyChildWebViewer(childWindowId.current);
          } catch (forceErr) {
            console.error('Force cleanup also failed:', forceErr);
          }
        }
      }
    };

    return () => {
      cleanup();
    };
  }, [childWindowCreated, webViewerContext, unifiedSidecarContext]);

  // Update child window bounds when container resizes (throttled)
  useEffect(() => {
    if (!childWindowCreated || !containerRef.current) return;

    const throttledUpdateBounds = () => {
      // Clear any pending update
      if (updateBoundsTimeoutRef.current) {
        clearTimeout(updateBoundsTimeoutRef.current);
      }
      
      // Schedule new update (throttled to max 60fps)
      updateBoundsTimeoutRef.current = setTimeout(() => {
        if (containerRef.current && childWindowId.current) {
          const rect = containerRef.current.getBoundingClientRect();
          
          // Use exact container bounds for child window positioning
          const adjustedBounds = {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(Math.max(rect.width, 300)), // Minimum width
            height: Math.round(Math.max(rect.height, 200)), // Minimum height
          };
          
          // Validate bounds are positive and reasonable
          if (adjustedBounds.width > 0 && adjustedBounds.height > 0 && 
              adjustedBounds.x >= 0 && adjustedBounds.y >= 0) {
            console.log(`[WebViewer-${childWindowId.current}] Updating child window bounds:`, adjustedBounds);
            window.electron.updateChildWebViewerBounds(childWindowId.current, adjustedBounds).catch(console.error);
          } else {
            console.warn(`[WebViewer-${childWindowId.current}] Invalid bounds calculated:`, adjustedBounds);
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
      if (containerRef.current && childWindowId.current) {
        const rect = containerRef.current.getBoundingClientRect();
        // Only update if position has actually changed
        const currentBounds = {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(Math.max(rect.height, 200)),
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
          
          console.log(`[WebViewer-${childWindowId.current}] Position changed, updating bounds:`, currentBounds);
          containerRef.current.dataset.lastBounds = JSON.stringify(currentBounds);
          window.electron.updateChildWebViewerBounds(childWindowId.current, currentBounds).catch(console.error);
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
  }, [childWindowCreated]);

  // Navigate to initial URL when child window is ready
  useEffect(() => {
    if (childWindowCreated && url) {
      navigateToUrl(url);
    }
  }, [childWindowCreated]);

  // Update navigation state periodically
  useEffect(() => {
    if (!childWindowCreated) return;

    const updateNavigationState = async () => {
      try {
        const state = await window.electron.getChildWebViewerNavigationState(childWindowId.current);
        if (state) {
          setCanGoBack(state.canGoBack);
          setCanGoForward(state.canGoForward);
          setIsLoading(state.isLoading);
          
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
  }, [childWindowCreated, actualUrl]);

  // Handle visibility changes (show/hide child window)
  useEffect(() => {
    if (!childWindowCreated) return;

    if (isVisible) {
      window.electron.showChildWebViewer(childWindowId.current);
    } else {
      window.electron.hideChildWebViewer(childWindowId.current);
    }
  }, [isVisible, childWindowCreated]);

  // Listen for child window events from main process
  useEffect(() => {
    const handleChildWindowLoading = (event: any, viewerId: string, loading: boolean) => {
      if (viewerId === childWindowId.current) {
        setIsLoading(loading);
      }
    };

    const handleChildWindowNavigation = (event: any, viewerId: string, navData: any) => {
      if (viewerId === childWindowId.current) {
        setActualUrl(navData.url);
        setInputUrl(navData.url);
        setPageTitle(navData.title);
        setCanGoBack(navData.canGoBack);
        setCanGoForward(navData.canGoForward);
        setIsSecure(navData.url.startsWith('https://'));
      }
    };

    const handleChildWindowError = (event: any, viewerId: string, errorMessage: string) => {
      if (viewerId === childWindowId.current) {
        setError(errorMessage);
        setIsLoading(false);
      }
    };

    const handleChildWindowTitle = (event: any, viewerId: string, title: string) => {
      if (viewerId === childWindowId.current) {
        setPageTitle(title);
      }
    };

    // Register event listeners
    window.electron.on('child-webviewer-loading', handleChildWindowLoading);
    window.electron.on('child-webviewer-navigation', handleChildWindowNavigation);
    window.electron.on('child-webviewer-error', handleChildWindowError);
    window.electron.on('child-webviewer-title', handleChildWindowTitle);

    return () => {
      // Cleanup event listeners
      window.electron.off('child-webviewer-loading', handleChildWindowLoading);
      window.electron.off('child-webviewer-navigation', handleChildWindowNavigation);
      window.electron.off('child-webviewer-error', handleChildWindowError);
      window.electron.off('child-webviewer-title', handleChildWindowTitle);
    };
  }, []);

  const navigateToUrl = async (targetUrl: string) => {
    if (!childWindowCreated) return;

    setIsLoading(true);
    setError(null);

    try {
      const success = await window.electron.childWebViewerNavigate(childWindowId.current, targetUrl);
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
    if (!childWindowCreated) return;
    
    try {
      await window.electron.childWebViewerRefresh(childWindowId.current);
    } catch (err) {
      console.error('Refresh error:', err);
    }
  };

  const handleOpenInBrowser = () => {
    window.electron.openExternal(actualUrl).catch(console.error);
  };

  const handleGoBack = async () => {
    if (!childWindowCreated || !canGoBack) return;
    
    try {
      await window.electron.childWebViewerGoBack(childWindowId.current);
    } catch (err) {
      console.error('Go back error:', err);
    }
  };

  const handleGoForward = async () => {
    if (!childWindowCreated || !canGoForward) return;
    
    try {
      await window.electron.childWebViewerGoForward(childWindowId.current);
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

      {/* Child Window Container */}
      <div 
        ref={containerRef}
        className="flex-1 relative overflow-hidden bg-background-default rounded-b-lg"
        style={{ 
          isolation: 'isolate', // Create new stacking context
          zIndex: 1 // Ensure it's below UI controls
        }}
      >
        {!childWindowCreated && (
          <div className="absolute inset-0 bg-background-default flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-text-subtle text-sm">Initializing child window...</p>
            </div>
          </div>
        )}
        
        {childWindowCreated && isLoading && (
          <div className="absolute top-0 left-0 right-0 h-1 bg-blue-200">
            <div className="h-full bg-blue-500 animate-pulse"></div>
          </div>
        )}
        
        {pageTitle && (
          <div className="absolute top-2 left-2 bg-black/70 text-white text-xs px-2 py-1 rounded opacity-0 hover:opacity-100 transition-opacity pointer-events-none">
            {pageTitle}
          </div>
        )}

        {/* Placeholder content - the actual web content is rendered in the child window */}
        {childWindowCreated && (
          <div className="absolute inset-0 bg-transparent flex items-center justify-center pointer-events-none">
            <div className="text-center text-text-subtle text-sm opacity-50">
              <p>Web content is displayed in a separate window</p>
              <p className="text-xs mt-1">Child window ID: {childWindowId.current}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default WebViewer;
