import { createContext, useContext, useRef, useCallback, useState } from 'react';

interface BrowserState {
  isOpen: boolean;
  url: string;
}

interface BrowserContextType {
  state: BrowserState;
  webviewRef: React.RefObject<Electron.WebviewTag | null>;
  openUrl: (url: string) => void;
  close: () => void;
  navigate: (url: string) => void;
  executeCommand: (command: string, params: Record<string, unknown>) => Promise<{ success: boolean; data?: unknown }>;
}

const BrowserContext = createContext<BrowserContextType | null>(null);

export function useBrowser() {
  const ctx = useContext(BrowserContext);
  if (!ctx) {
    throw new Error('useBrowser must be used within a BrowserProvider');
  }
  return ctx;
}

export function useBrowserOptional() {
  return useContext(BrowserContext);
}

export function BrowserProvider({ children }: { children: React.ReactNode }) {
  const webviewRef = useRef<Electron.WebviewTag | null>(null);
  const [state, setState] = useState<BrowserState>({ isOpen: false, url: '' });

  const openUrl = useCallback((url: string) => {
    setState({ isOpen: true, url });
  }, []);

  const close = useCallback(() => {
    setState({ isOpen: false, url: '' });
  }, []);

  const navigate = useCallback((url: string) => {
    const wv = webviewRef.current;
    if (wv) {
      wv.loadURL(url).catch((err: Error) => {
        // ERR_ABORTED happens on redirects — not a real error
        if (err.message?.includes('ERR_ABORTED')) return;
        console.error('Navigation failed:', err);
      });
      setState((prev) => ({ ...prev, url }));
    }
  }, []);

  const executeCommand = useCallback(
    async (command: string, params: Record<string, unknown>): Promise<{ success: boolean; data?: unknown }> => {
      if (command === 'open') {
        const url = params.url as string;
        openUrl(url);
        return { success: true };
      }

      if (command === 'close') {
        close();
        return { success: true };
      }

      if (command === 'navigate') {
        const url = params.url as string;
        navigate(url);
        return { success: true };
      }

      const wv = webviewRef.current;
      if (!wv) {
        return { success: false, data: 'Browser not open' };
      }

      switch (command) {
        case 'screenshot': {
          // webview.capturePage returns a NativeImage
          const image = await wv.capturePage();
          return { success: true, data: image.toDataURL() };
        }

        case 'click': {
          const selector = params.selector as string;
          await wv.executeJavaScript(`
            (function() {
              const el = document.querySelector(${JSON.stringify(selector)});
              if (!el) throw new Error('Element not found: ${selector}');
              el.click();
              return true;
            })()
          `);
          return { success: true };
        }

        case 'type': {
          const selector = params.selector as string;
          const text = params.text as string;
          await wv.executeJavaScript(`
            (function() {
              const el = document.querySelector(${JSON.stringify(selector)});
              if (!el) throw new Error('Element not found: ${selector}');
              el.focus();
              el.value = ${JSON.stringify(text)};
              el.dispatchEvent(new Event('input', { bubbles: true }));
              el.dispatchEvent(new Event('change', { bubbles: true }));
              return true;
            })()
          `);
          return { success: true };
        }

        case 'get_text': {
          const selector = params.selector as string;
          const text = await wv.executeJavaScript(`
            (function() {
              const el = document.querySelector(${JSON.stringify(selector)});
              if (!el) throw new Error('Element not found: ${selector}');
              return el.innerText || el.textContent || '';
            })()
          `);
          return { success: true, data: text };
        }

        case 'get_html': {
          const selector = params.selector as string;
          const html = await wv.executeJavaScript(`
            (function() {
              const el = document.querySelector(${JSON.stringify(selector)});
              if (!el) throw new Error('Element not found: ${selector}');
              return el.innerHTML;
            })()
          `);
          return { success: true, data: html };
        }

        case 'evaluate': {
          const script = params.script as string;
          const result = await wv.executeJavaScript(script);
          return { success: true, data: result };
        }

        case 'wait': {
          const selector = params.selector as string;
          const timeout = (params.timeout as number) || 5000;
          const startTime = Date.now();

          while (Date.now() - startTime < timeout) {
            const found = await wv.executeJavaScript(
              `!!document.querySelector(${JSON.stringify(selector)})`
            );
            if (found) {
              return { success: true };
            }
            await new Promise((resolve) => setTimeout(resolve, 100));
          }
          return { success: false, data: `Timeout waiting for element: ${selector}` };
        }

        case 'scroll': {
          const x = (params.x as number) || 0;
          const y = (params.y as number) || 0;
          await wv.executeJavaScript(`window.scrollBy(${x}, ${y})`);
          return { success: true };
        }

        default:
          return { success: false, data: `Unknown command: ${command}` };
      }
    },
    [openUrl, close, navigate]
  );

  return (
    <BrowserContext.Provider value={{ state, webviewRef, openUrl, close, navigate, executeCommand }}>
      {children}
    </BrowserContext.Provider>
  );
}
