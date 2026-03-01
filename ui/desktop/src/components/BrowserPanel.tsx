import { useEffect, useRef, useState, useCallback } from 'react';
import { useBrowser } from './BrowserContext';

export default function BrowserPanel() {
  const { state, webviewRef, close } = useBrowser();
  const containerRef = useRef<HTMLDivElement>(null);
  const [displayUrl, setDisplayUrl] = useState(state.url);
  const [isLoading, setIsLoading] = useState(false);
  const [panelWidth, setPanelWidth] = useState(500);
  const isDragging = useRef(false);

  useEffect(() => {
    setDisplayUrl(state.url);
  }, [state.url]);

  // Attach webview event listeners once the element mounts
  useEffect(() => {
    if (!state.isOpen) return;

    const container = containerRef.current;
    if (!container) return;

    // The webview element is rendered declaratively but we need to grab it
    const wv = container.querySelector('webview') as Electron.WebviewTag | null;
    if (!wv) return;

    webviewRef.current = wv;

    const onLoadStart = () => setIsLoading(true);
    const onLoadStop = () => setIsLoading(false);
    const onNavigate = (e: Electron.DidNavigateEvent) => setDisplayUrl(e.url);
    const onFailLoad = (e: Electron.DidFailLoadEvent) => {
      // ERR_ABORTED (-3) happens on redirects — not a real error
      if (e.errorCode === -3) return;
      console.error(`Webview load failed: ${e.errorDescription} (${e.errorCode})`);
      setIsLoading(false);
    };

    wv.addEventListener('did-start-loading', onLoadStart);
    wv.addEventListener('did-stop-loading', onLoadStop);
    wv.addEventListener('did-navigate', onNavigate as EventListener);
    wv.addEventListener('did-navigate-in-page', onNavigate as EventListener);
    wv.addEventListener('did-fail-load', onFailLoad as EventListener);

    return () => {
      wv.removeEventListener('did-start-loading', onLoadStart);
      wv.removeEventListener('did-stop-loading', onLoadStop);
      wv.removeEventListener('did-navigate', onNavigate as EventListener);
      wv.removeEventListener('did-navigate-in-page', onNavigate as EventListener);
      wv.removeEventListener('did-fail-load', onFailLoad as EventListener);
      if (webviewRef.current === wv) {
        webviewRef.current = null;
      }
    };
  }, [state.isOpen, state.url, webviewRef]);

  // Resize drag handler
  const onResizeMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      isDragging.current = true;
      const startX = e.clientX;
      const startWidth = panelWidth;

      const onMouseMove = (moveEvent: MouseEvent) => {
        if (!isDragging.current) return;
        const delta = startX - moveEvent.clientX;
        const newWidth = Math.max(300, Math.min(1200, startWidth + delta));
        setPanelWidth(newWidth);
      };

      const onMouseUp = () => {
        isDragging.current = false;
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
      };

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    },
    [panelWidth]
  );

  const handleUrlSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const wv = webviewRef.current;
    if (wv && displayUrl) {
      let url = displayUrl;
      if (!url.startsWith('http://') && !url.startsWith('https://')) {
        url = 'https://' + url;
      }
      wv.loadURL(url).catch((err: Error) => {
        if (err.message?.includes('ERR_ABORTED')) return;
        console.error('URL load failed:', err);
      });
    }
  };

  if (!state.isOpen) return null;

  return (
    <div
      ref={containerRef}
      className="relative flex flex-col border-l border-border-subtle bg-surface-default"
      style={{ width: panelWidth, minWidth: 300 }}
    >
      {/* Resize handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-accent-default/30 z-10"
        onMouseDown={onResizeMouseDown}
      />

      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-border-subtle bg-surface-raised">
        <form onSubmit={handleUrlSubmit} className="flex-1 flex">
          <input
            type="text"
            value={displayUrl}
            onChange={(e) => setDisplayUrl(e.target.value)}
            className="flex-1 text-sm px-2 py-1 rounded bg-surface-default border border-border-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-default"
            placeholder="Enter URL..."
          />
        </form>
        {isLoading && (
          <div className="w-4 h-4 border-2 border-accent-default border-t-transparent rounded-full animate-spin" />
        )}
        <button
          onClick={close}
          className="p-1 rounded hover:bg-surface-hover text-text-muted hover:text-text-default"
          title="Close browser"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M4 4L12 12M12 4L4 12"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>

      {/* Webview */}
      <div className="flex-1">
        {/* eslint-disable-next-line react/no-unknown-property */}
        <webview
          src={state.url}
          style={{ width: '100%', height: '100%' }}
          /* @ts-expect-error webview is an Electron-specific element */
          allowpopups="true"
        />
      </div>
    </div>
  );
}
