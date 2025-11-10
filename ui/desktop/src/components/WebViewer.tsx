import { useRef, useState, useEffect } from 'react';
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

  // If it doesn't start with http:// or https://, add https://
  if (allowAllSites && !trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
    // Check if it looks like a domain (contains dots but not localhost)
    if (trimmed.includes('.') && !trimmed.startsWith('localhost')) {
      return `https://${trimmed}`;
    }
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
  const [retryCount, setRetryCount] = useState(0);
  const [iframeReady, setIframeReady] = useState(false);
  const [isSecure, setIsSecure] = useState(false);
  const [isBlocked, setIsBlocked] = useState(false);
  // eslint-disable-next-line no-undef
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (onUrlChange) {
      onUrlChange(url);
    }
    
    // Check if URL is secure
    setIsSecure(url.startsWith('https://'));
  }, [url, onUrlChange]);

  // For localhost URLs, poll the server until it's ready
  useEffect(() => {
    const isLocalhost = isLocalhostUrl(url);
    
    if (!isLocalhost || iframeReady) {
      // For non-localhost URLs or if already ready, show iframe immediately
      if (!isLocalhost) {
        setIframeReady(true);
      }
      return;
    }

    setIsLoading(true);
    let mounted = true;
    let pollInterval: ReturnType<typeof setInterval> | null = null;
    let attemptCount = 0;
    const maxAttempts = 10;

    const checkServerReady = async () => {
      attemptCount++;

      try {
        // Try to fetch from the URL to see if server is responding
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 2000); // 2 second timeout

        await fetch(url, {
          method: 'HEAD',
          signal: controller.signal,
          mode: 'no-cors', // Use no-cors to avoid CORS issues during check
        });

        window.clearTimeout(timeoutId);

        console.log(`Server at ${url} is ready`);

        if (mounted) {
          setIframeReady(true);
          if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
          }
        }
      } catch (error) {
        console.log(`Server not ready yet (attempt ${attemptCount}/${maxAttempts}):`, error);

        if (attemptCount >= maxAttempts) {
          console.log('Max attempts reached, showing iframe anyway');
          if (mounted) {
            setIframeReady(true);
            if (pollInterval) {
              clearInterval(pollInterval);
              pollInterval = null;
            }
          }
        }
      }
    };

    // Initial delay to let server start
    const initialTimer = setTimeout(() => {
      if (!mounted) return;

      // First check
      checkServerReady();

      // Set up polling
      pollInterval = setInterval(() => {
        if (mounted && !iframeReady) {
          checkServerReady();
        }
      }, 1000); // Poll every second
    }, 800); // Wait 800ms before first check

    return () => {
      mounted = false;
      window.clearTimeout(initialTimer);
      if (pollInterval) {
        window.clearInterval(pollInterval);
      }
    };
  }, [url, iframeReady]);

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
    setIsLoading(true);
    setRetryCount(0);
    setIframeReady(false);
    setIsBlocked(false);

    // Save to localStorage
    if (typeof window !== 'undefined') {
      localStorage.setItem(storageKey, formattedUrl);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleUrlSubmit(inputUrl);
    }
  };

  const handleRefresh = () => {
    if (iframeRef.current) {
      setIsLoading(true);
      const currentSrc = iframeRef.current.src;
      iframeRef.current.src = '';
      iframeRef.current.src = currentSrc;
    }
  };

  const handleOpenInBrowser = () => {
    window.open(url, '_blank');
  };

  const handleGoBack = () => {
    if (iframeRef.current?.contentWindow) {
      try {
        iframeRef.current.contentWindow.history.back();
      } catch (e) {
        console.warn('Cannot access iframe history:', e);
      }
    }
  };

  const handleGoForward = () => {
    if (iframeRef.current?.contentWindow) {
      try {
        iframeRef.current.contentWindow.history.forward();
      } catch (e) {
        console.warn('Cannot access iframe history:', e);
      }
    }
  };

  const handleHome = () => {
    const homeUrl = allowAllSites ? 'https://google.com' : 'http://localhost:3000';
    handleUrlSubmit(homeUrl);
  };

  const handleIframeLoad = () => {
    setIsLoading(false);
    setError(null);
    setRetryCount(0);
    setIsBlocked(false);

    if (retryTimeoutRef.current) {
      window.clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = null;
    }

    // Try to update navigation state (may not work due to CORS)
    try {
      if (iframeRef.current?.contentWindow) {
        setCanGoBack(iframeRef.current.contentWindow.history.length > 1);
        setCanGoForward(false);
      }
    } catch (e) {
      // Ignore CORS errors
    }
  };

  const handleIframeError = () => {
    console.log('Iframe error occurred for URL:', url);
    setIsLoading(false);
    setIsBlocked(true);
    
    // Check if this might be a blocking issue
    if (!isLocalhostUrl(url)) {
      setError(
        `This website (${domain}) cannot be displayed in an embedded viewer due to security restrictions. This is common for many websites to prevent clickjacking attacks.`
      );
    } else {
      const maxRetries = 3;

      if (retryCount < maxRetries) {
        const retryDelay = Math.min(1000 * Math.pow(2, retryCount), 5000);

        console.log(
          `Retrying to load ${url} (attempt ${retryCount + 1}/${maxRetries}) in ${retryDelay}ms...`
        );

        if (retryTimeoutRef.current) {
          window.clearTimeout(retryTimeoutRef.current);
        }

        retryTimeoutRef.current = setTimeout(() => {
          setRetryCount((prev) => prev + 1);
          if (iframeRef.current) {
            const currentSrc = iframeRef.current.src;
            iframeRef.current.src = '';
            iframeRef.current.src = currentSrc;
          }
        }, retryDelay);
      } else {
        setError('Failed to load localhost server. Make sure the server is running and accessible.');
      }
    }
  };

  useEffect(() => {
    return () => {
      if (retryTimeoutRef.current) {
        window.clearTimeout(retryTimeoutRef.current);
      }
    };
  }, []);

  const domain = getDomainFromUrl(url);
  const isLocalhost = isLocalhostUrl(url);

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
          {isBlocked && !isLocalhost && (
            <div className="mt-2 pt-2 border-t border-red-200 dark:border-red-800">
              <p className="text-red-700 dark:text-red-300 text-xs mb-2">Try these alternatives:</p>
              <div className="flex gap-2 flex-wrap">
                <Button
                  onClick={handleOpenInBrowser}
                  size="sm"
                  variant="outline"
                  className="text-xs"
                >
                  <ExternalLink className="w-3 h-3 mr-1" />
                  Open in Browser
                </Button>
                <Button
                  onClick={() => handleUrlSubmit('https://google.com')}
                  size="sm"
                  variant="outline"
                  className="text-xs"
                >
                  Try Google
                </Button>
                <Button
                  onClick={() => handleUrlSubmit('https://wikipedia.org')}
                  size="sm"
                  variant="outline"
                  className="text-xs"
                >
                  Try Wikipedia
                </Button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Iframe Content */}
      <div className="flex-1 relative overflow-hidden">
        {iframeReady && !isBlocked && (
          <iframe
            ref={iframeRef}
            src={url}
            className="w-full h-full border-0"
            title={allowAllSites ? "Web Viewer" : "Localhost Viewer"}
            onLoad={handleIframeLoad}
            onError={handleIframeError}
            sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-presentation allow-top-navigation-by-user-activation"
          />
        )}

        {/* Blocked content fallback */}
        {isBlocked && !isLocalhost && (
          <div className="absolute inset-0 bg-background-default flex items-center justify-center">
            <div className="text-center max-w-md p-6">
              <div className="w-16 h-16 mx-auto mb-4 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
                <ShieldOff className="w-8 h-8 text-red-500" />
              </div>
              <h3 className="text-lg font-semibold text-textStandard mb-2">Website Blocked</h3>
              <p className="text-textSubtle text-sm mb-4">
                {domain} prevents embedding for security reasons. This is normal for many websites.
              </p>
              <div className="space-y-2">
                <Button
                  onClick={handleOpenInBrowser}
                  className="w-full"
                  variant="default"
                >
                  <ExternalLink className="w-4 h-4 mr-2" />
                  Open {domain} in Browser
                </Button>
                <div className="text-xs text-textSubtle">
                  Or try a different website that allows embedding
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Loading overlay */}
        {(!iframeReady || isLoading) && !isBlocked && (
          <div className="absolute inset-0 bg-background-default/80 flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-textSubtle text-sm">
                {!iframeReady ? 'Initializing...' : `Loading ${domain}...`}
                {retryCount > 0 && ` (retry ${retryCount}/3)`}
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default WebViewer;
