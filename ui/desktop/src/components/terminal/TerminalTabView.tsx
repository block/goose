import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

type TerminalTabViewProps = {
  sessionId: string;
  tabId: string;
  cwd: string;
  isActive: boolean;
  focusToken: number;
};

function isDarkTheme(): boolean {
  if (typeof document === 'undefined') return false;
  return (
    document.documentElement.classList.contains('dark') ||
    document.documentElement.getAttribute('data-theme') === 'dark' ||
    window.matchMedia('(prefers-color-scheme: dark)').matches
  );
}

export function TerminalTabView({
  sessionId,
  tabId,
  cwd,
  isActive,
  focusToken,
}: TerminalTabViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current || termRef.current) return;

    const dark = isDarkTheme();
    const term = new Terminal({
      cursorBlink: true,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
      fontSize: 13,
      theme: dark
        ? {
            background: '#1a1a1a',
            foreground: '#e8e8e8',
            cursor: '#e8e8e8',
            selectionBackground: '#ffffff33',
          }
        : {
            background: '#fafafa',
            foreground: '#1a1a1a',
            cursor: '#1a1a1a',
            selectionBackground: '#00000022',
          },
      allowProposedApi: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(containerRef.current);
    fit.fit();

    termRef.current = term;
    fitRef.current = fit;

    const cols = term.cols;
    const rows = term.rows;

    void window.electron.terminalCreate({ sessionId, tabId, cwd, cols, rows }).then((result) => {
      if (!result.ok) {
        term.writeln(`\r\n\x1b[31mFailed to start terminal: ${result.error}\x1b[0m\r\n`);
      }
    });

    const disposable = term.onData((data) => {
      void window.electron.terminalWrite({ sessionId, tabId, data });
    });

    const onData = (_event: unknown, payload: unknown) => {
      const msg = payload as { sessionId?: string; tabId?: string; data?: string };
      if (msg.sessionId !== sessionId || msg.tabId !== tabId || typeof msg.data !== 'string') {
        return;
      }
      term.write(msg.data);
    };

    const onExit = (_event: unknown, payload: unknown) => {
      const msg = payload as { sessionId?: string; tabId?: string; exitCode?: number };
      if (msg.sessionId !== sessionId || msg.tabId !== tabId) return;
      term.writeln(`\r\n\x1b[90m[process exited with code ${msg.exitCode ?? '?'}]\x1b[0m`);
    };

    window.electron.on('terminal-data', onData);
    window.electron.on('terminal-exit', onExit);

    const ro = new ResizeObserver(() => {
      if (!fitRef.current || !termRef.current) return;
      try {
        fitRef.current.fit();
        void window.electron.terminalResize({
          sessionId,
          tabId,
          cols: termRef.current.cols,
          rows: termRef.current.rows,
        });
      } catch {
        // ignore fit races while hidden
      }
    });
    ro.observe(containerRef.current);

    return () => {
      ro.disconnect();
      disposable.dispose();
      window.electron.off('terminal-data', onData);
      window.electron.off('terminal-exit', onExit);
      // Do not kill the PTY here — panel hide / tab remount must keep the
      // process alive. Explicit kill happens on tab close or session delete.
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
    // Create once per tab mount; cwd is only used at spawn.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, tabId]);

  useEffect(() => {
    if (!isActive || !fitRef.current || !termRef.current) return;
    try {
      fitRef.current.fit();
      void window.electron.terminalResize({
        sessionId,
        tabId,
        cols: termRef.current.cols,
        rows: termRef.current.rows,
      });
    } catch {
      // ignore
    }
  }, [isActive, sessionId, tabId]);

  useEffect(() => {
    if (!isActive || focusToken === 0) return;
    termRef.current?.focus();
  }, [isActive, focusToken]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full min-h-0 overflow-hidden"
      style={{ display: isActive ? 'block' : 'none' }}
    />
  );
}
