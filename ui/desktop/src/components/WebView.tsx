import React, { useEffect, useRef, useState } from 'react';

interface WebViewProps {
  isVisible: boolean;
  onClose: () => void;
  url?: string;
}

const WebView: React.FC<WebViewProps> = ({ url: initialUrl, isVisible, onClose }) => {
  const webviewRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [webviewCreated, setWebviewCreated] = useState(false);

  // Get the working directory from appConfig and encode it for the URL
  const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') || '';
  const encodedWorkingDir = encodeURIComponent(workingDir);

  // Fixed URL for the code-server with the folder parameter added
  const webViewUrl = `http://127.0.0.1:8000?tkn=43&folder=${encodedWorkingDir}`;

  // Create the webview when the component mounts
  useEffect(() => {
    // Only proceed if we have the DOM reference
    if (!webviewRef.current) return;

    // If webview doesn't exist yet, create it
    if (!webviewCreated) {
      console.log('Creating new webview with URL:', webViewUrl);

      // Clear any existing content
      webviewRef.current.innerHTML = '';

      // Create a new webview element
      const webview = document.createElement('webview');

      // Set attributes
      webview.src = webViewUrl;
      webview.style.width = '100%';
      webview.style.height = '100%';
      webview.style.border = 'none';

      // Add webPreferences to disable security features for localhost
      webview.setAttribute('webpreferences', 'contextIsolation=no, nodeIntegration=yes');

      // Add event listener for load completion
      webview.addEventListener('dom-ready', () => {
        console.log('WebView loaded:', webViewUrl);
        setIsLoading(false);
        setWebviewCreated(true);
      });

      // Add error handler
      webview.addEventListener('did-fail-load', (event) => {
        console.error('WebView failed to load:', event);
        setLoadError(`Failed to load: ${event.errorDescription || 'Unknown error'}`);
        setIsLoading(false);
      });

      // Append the webview to our container
      webviewRef.current.appendChild(webview);
    }
  }, [webviewCreated, webViewUrl]);

  // Handle visibility changes
  useEffect(() => {
    if (webviewRef.current) {
      const webview = webviewRef.current.querySelector('webview');
      if (webview) {
        console.log(`Setting webview visibility to ${isVisible ? 'visible' : 'hidden'}`);
        if (isVisible) {
          // When showing, make sure it's loaded
          (webview as any).reload();
          setIsLoading(true);
        }
      }
    }
  }, [isVisible, webViewUrl]);

  return (
    <div ref={containerRef} className="h-full w-full bg-bgApp flex flex-col">
      <div className="flex justify-between items-center p-2 border-b border-borderSubtle">
        <div className="text-sm font-medium text-textStandard overflow-hidden text-ellipsis whitespace-nowrap max-w-[calc(100%-80px)] flex items-center">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="mr-2"
          >
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
            <line x1="8" y1="21" x2="16" y2="21"></line>
            <line x1="12" y1="17" x2="12" y2="21"></line>
          </svg>
          Code Editor
        </div>
        <div className="flex items-center">
          <button
            onClick={() => {
              // Force reload the webview
              if (webviewRef.current) {
                const webview = webviewRef.current.querySelector('webview');
                if (webview) {
                  setIsLoading(true);
                  setLoadError(null);
                  (webview as any).reload();
                }
              }
            }}
            className="rounded-full p-1 hover:bg-bgSubtle text-textStandard mr-1"
            title="Reload"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"></path>
              <path d="M21 3v5h-5"></path>
              <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"></path>
              <path d="M8 16H3v5"></path>
            </svg>
          </button>
          <button
            onClick={onClose}
            className="rounded-full p-1 hover:bg-bgSubtle text-textStandard"
            title="Close"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>
      <div className="flex-1 relative">
        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-bgApp bg-opacity-75 z-10">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
          </div>
        )}
        {loadError && (
          <div className="absolute inset-0 flex items-center justify-center bg-bgApp z-10 p-4">
            <div className="bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200 p-4 rounded-md max-w-md">
              <h3 className="font-medium mb-2">Error Loading WebView</h3>
              <p>{loadError}</p>
              <button
                onClick={() => {
                  // Force reload the webview
                  if (webviewRef.current) {
                    const webview = webviewRef.current.querySelector('webview');
                    if (webview) {
                      setIsLoading(true);
                      setLoadError(null);
                      (webview as any).reload();
                    } else {
                      // If webview element doesn't exist, create it
                      setWebviewCreated(false); // This will trigger recreation
                    }
                  }
                }}
                className="mt-4 px-4 py-2 bg-red-200 dark:bg-red-800 text-red-800 dark:text-red-200 rounded hover:bg-red-300 dark:hover:bg-red-700 transition-colors"
              >
                Try Again
              </button>
            </div>
          </div>
        )}
        <div ref={webviewRef} className="h-full"></div>
      </div>
    </div>
  );
};

export default WebView;
