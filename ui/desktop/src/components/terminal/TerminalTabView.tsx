import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

type TerminalTabViewProps = {
  sessionId: string;
  tabId: string;
  cwd: string;
  isActive: boolean;
  isPanelOpen: boolean;
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
  isPanelOpen,
  focusToken,
}: TerminalTabViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const visibleRef = useRef(false);
  visibleRef.current = isActive && isPanelOpen;

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
      scrollback: 5000,
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

    const fitAndResize = () => {
      if (!visibleRef.current || !fitRef.current || !termRef.current || !containerRef.current) {
        return;
      }
      const { clientWidth, clientHeight } = containerRef.current;
      if (clientWidth < 16 || clientHeight < 16) return;
      try {
        fitRef.current.fit();
        const nextCols = termRef.current.cols;
        const nextRows = termRef.current.rows;
        if (nextCols < 2 || nextRows < 1) return;
        void window.electron.terminalResize({
          sessionId,
          tabId,
          cols: nextCols,
          rows: nextRows,
        });
      } catch {
        // ignore fit races while collapsing
      }
    };

    const ro = new ResizeObserver(() => {
      fitAndResize();
    });
    ro.observe(containerRef.current);

    return () => {
      ro.disconnect();
      disposable.dispose();
      window.electron.off('terminal-data', onData);
      window.electron.off('terminal-exit', onExit);
      // Do not kill the PTY here — panel collapse must keep process + scrollback.
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
    // Create once per tab mount; cwd is only used at spawn.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, tabId]);

  useEffect(() => {
    if (!isActive || !isPanelOpen || !fitRef.current || !termRef.current) return;
    const id = requestAnimationFrame(() => {
      if (!fitRef.current || !termRef.current || !containerRef.current) return;
      const { clientWidth, clientHeight } = containerRef.current;
      if (clientWidth < 16 || clientHeight < 16) return;
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
    });
    return () => cancelAnimationFrame(id);
  }, [isActive, isPanelOpen, sessionId, tabId]);

  useEffect(() => {
    if (!isActive || !isPanelOpen || focusToken === 0) return;
    termRef.current?.focus();
  }, [isActive, isPanelOpen, focusToken]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full min-h-0 overflow-hidden"
      // Keep inactive tabs in the layout tree without display:none so xterm
      // does not tear down its renderer when switching tabs.
      style={{
        position: isActive ? 'relative' : 'absolute',
        inset: 0,
        visibility: isActive ? 'visible' : 'hidden',
        pointerEvents: isActive && isPanelOpen ? 'auto' : 'none',
        zIndex: isActive ? 1 : 0,
      }}
    />
  );
}
