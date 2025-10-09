import { useRef, useState, useEffect } from 'react';
import { RefreshCw, ExternalLink, ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';

interface LocalhostViewerProps {
  initialUrl?: string;
  onUrlChange?: (url: string) => void;
}

function isValidLocalhostUrl(url: string): boolean {
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

function formatUrl(input: string): string {
  // If it's just a port number, prepend localhost
  if (/^\d+$/.test(input.trim())) {
    return `http://localhost:${input.trim()}`;
  }

  // If it starts with localhost: or 127.0.0.1: without protocol
  if (/^(localhost|127\.0\.0\.1):\d+/.test(input.trim())) {
    return `http://${input.trim()}`;
  }

  return input.trim();
}

export function LocalhostViewer({
  initialUrl = 'http://localhost:3000',
  onUrlChange,
}: LocalhostViewerProps) {
  // Initialize from localStorage or use initialUrl
  const [url, setUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('goose-sidecar-url') || initialUrl;
    }
    return initialUrl;
  });

  const [inputUrl, setInputUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('goose-sidecar-url') || initialUrl;
    }
    return initialUrl;
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canGoBack, setCanGoBack] = useState(false);
  const [canGoForward, setCanGoForward] = useState(false);
  const [retryCount, setRetryCount] = useState(0);
  const [iframeReady, setIframeReady] = useState(false);
  // eslint-disable-next-line no-undef
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (onUrlChange) {
      onUrlChange(url);
    }
  }, [url, onUrlChange]);

  // Poll the server until it's ready before showing iframe
  useEffect(() => {
    if (iframeReady) return; // Don't run if already ready

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

        // If we get here without throwing, server is likely ready
        // Note: with mode: 'no-cors', we can't read the response but no error means connection succeeded
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
          // Give up and show the iframe anyway - let the user manually refresh if needed
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
  }, [url, iframeReady]); // Re-run when URL changes or iframeReady resets

  const handleUrlSubmit = (newUrl: string) => {
    const formattedUrl = formatUrl(newUrl);

    if (!isValidLocalhostUrl(formattedUrl)) {
      setError('Only localhost URLs are allowed (e.g., http://localhost:3000)');
      return;
    }

    setError(null);
    setUrl(formattedUrl);
    setInputUrl(formattedUrl);
    setIsLoading(true);
    setRetryCount(0); // Reset retry count when changing URL
    setIframeReady(false); // Reset iframe ready state to trigger re-check

    // Save to localStorage
    if (typeof window !== 'undefined') {
      localStorage.setItem('goose-sidecar-url', formattedUrl);
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
      // Create a new URL to force refresh
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

  const handleIframeLoad = () => {
    setIsLoading(false);
    setError(null);
    setRetryCount(0); // Reset retry count on successful load

    // Clear any pending retry timeout
    if (retryTimeoutRef.current) {
      window.clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = null;
    }

    // Try to update navigation state (may not work due to CORS)
    try {
      if (iframeRef.current?.contentWindow) {
        setCanGoBack(iframeRef.current.contentWindow.history.length > 1);
        setCanGoForward(false); // Reset forward state
      }
    } catch (e) {
      // Ignore CORS errors when trying to access iframe history
    }
  };

  const handleIframeError = () => {
    // Implement retry logic with exponential backoff
    const maxRetries = 3;

    if (retryCount < maxRetries) {
      const retryDelay = Math.min(1000 * Math.pow(2, retryCount), 5000); // Exponential backoff: 1s, 2s, 4s (max 5s)

      console.log(
        `Retrying to load ${url} (attempt ${retryCount + 1}/${maxRetries}) in ${retryDelay}ms...`
      );

      // Clear any existing retry timeout
      if (retryTimeoutRef.current) {
        window.clearTimeout(retryTimeoutRef.current);
      }

      retryTimeoutRef.current = setTimeout(() => {
        setRetryCount((prev) => prev + 1);
        if (iframeRef.current) {
          // Force reload by setting src
          const currentSrc = iframeRef.current.src;
          iframeRef.current.src = '';
          iframeRef.current.src = currentSrc;
        }
      }, retryDelay);
    } else {
      setIsLoading(false);
      setError(
        `Failed to load ${url} after ${maxRetries} attempts. Make sure the server is running.`
      );
    }
  };

  // Clean up retry timeout on unmount
  useEffect(() => {
    return () => {
      if (retryTimeoutRef.current) {
        window.clearTimeout(retryTimeoutRef.current);
      }
    };
  }, []);

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
        </div>

        {/* URL Input */}
        <div className="flex-1 flex items-center">
          <input
            type="text"
            value={inputUrl}
            onChange={(e) => setInputUrl(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="http://localhost:3000"
            className="flex-1 px-3 py-1 text-sm border border-borderSubtle rounded-md bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent focus:border-transparent"
          />
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
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="sm" onClick={handleOpenInBrowser} className="p-1 h-8 w-8">
              <ExternalLink size={14} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Open in Browser</TooltipContent>
        </Tooltip>
      </div>

      {/* Error Display */}
      {error && (
        <div className="p-3 bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800">
          <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
        </div>
      )}

      {/* Iframe Content */}
      <div className="flex-1 relative overflow-hidden">
        {iframeReady && (
          <iframe
            ref={iframeRef}
            src={url}
            className="w-full h-full border-0"
            title="Localhost Viewer"
            onLoad={handleIframeLoad}
            onError={handleIframeError}
            sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-presentation allow-top-navigation-by-user-activation"
          />
        )}

        {/* Loading overlay - show while waiting for iframe to be ready or while loading */}
        {(!iframeReady || isLoading) && (
          <div className="absolute inset-0 bg-background-default/80 flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-textSubtle text-sm">
                {!iframeReady ? 'Initializing...' : `Loading ${url}...`}
                {retryCount > 0 && ` (retry ${retryCount}/3)`}
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default LocalhostViewer;
