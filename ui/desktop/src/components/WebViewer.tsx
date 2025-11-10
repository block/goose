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
  // Generate a unique ID for this BrowserView instance
  const browserViewId = useRef(`webviewer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`);
  const containerRef = useRef<HTMLDivElement>(null);
  
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

  // Create BrowserView on mount
  useEffect(() => {
    const createBrowserView = async () => {
      try {
        const success = await window.electron.createBrowserView(browserViewId.current);
        if (success) {
          setBrowserViewCreated(true);
          console.log('BrowserView created:', browserViewId.current);
        } else {
          setError('Failed to create browser view');
        }
      } catch (err) {
        console.error('Error creating BrowserView:', err);
        setError('Failed to initialize browser view');
      }
    };

    createBrowserView();

    // Cleanup on unmount
    return () => {
      if (browserViewCreated) {
        window.electron.destroyBrowserView(browserViewId.current).catch(console.error);
      }
    };
  }, []);

  // Update BrowserView bounds when container resizes
  useEffect(() => {
    if (!browserViewCreated || !containerRef.current) return;

    const updateBounds = () => {
      if (containerRef.current) {
        const rect = containerRef.current.getBoundingClientRect();
        window.electron.updateBrowserViewBounds(browserViewId.current, {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        }).catch(console.error);
      }
    };

    // Initial bounds update
    updateBounds();

    // Set up resize observer
    const resizeObserver = new ResizeObserver(updateBounds);
    resizeObserver.observe(containerRef.current);

    // Also listen for window resize
    window.addEventListener('resize', updateBounds);

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener('resize', updateBounds);
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
    <div className="h-full flex flex-col bg-background-default">
      {/* URL Bar and Controls */}
      <div className="flex items-center gap-2 p-3 border-b border-borderSubtle bg-background-muted">
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
              className="w-full pl-8 pr-3 py-1 text-sm border border-borderSubtle rounded-md bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent focus:border-transparent"
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

        {/* Domain indicator and external link button */}
        <div className="flex items-center gap-2">
          {domain && (
            <div className="text-xs text-textSubtle bg-background-default px-2 py-1 rounded border border-borderSubtle">
              {domain}
            </div>
          )}
          
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
        className="flex-1 relative overflow-hidden bg-white"
      >
        {!browserViewCreated && (
          <div className="absolute inset-0 bg-background-default flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-textSubtle text-sm">Initializing browser...</p>
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
