import React, { useMemo } from 'react';
import { useSidecar } from './SidecarContext';
import MCPUIResourceRenderer from '../MCPUIResourceRenderer';

export default function SidecarPanel() {
  const { isOpen, content, close, widthPct } = useSidecar();

  const style = useMemo(() => {
    return isOpen
      ? {
          width: `${Math.min(75, Math.max(10, widthPct * 100))}%`,
          minWidth: 320,
        }
      : { width: 0 };
  }, [isOpen, widthPct]);

  return (
    <div
      className={`transition-[width,opacity] duration-200 ease-in-out bg-background-default border-l border-borderSubtle h-full ${
        isOpen ? 'opacity-100' : 'opacity-0'
      } overflow-hidden flex-shrink-0`}
      style={style as React.CSSProperties}
      aria-hidden={!isOpen}
    >
      {isOpen && (
        <div className="h-full flex flex-col">
          <div className="sticky top-0 z-100 flex items-center justify-between px-3 py-2 border-b border-borderSubtle bg-background-default/95 backdrop-blur supports-[backdrop-filter]:bg-background-default/70">
            <div className="text-xs font-sans text-textSubtle uppercase tracking-wide">
              MCP‑UI sidecar
            </div>
            <button
              className="no-drag inline-flex items-center justify-center w-6 h-6 cursor-pointer rounded hover:bg-bgSubtle text-textSubtle hover:text-textStandard relative z-50 pointer-events-auto"
              onClick={close}
              aria-label="Close side panel"
              title="Close"
            >
              ×
            </button>
          </div>
          <div className="flex-1 min-h-0 overflow-auto p-3">
            {content.kind === 'mcp-ui' && (
              <MCPUIResourceRenderer
                content={content.resource}
                appendPromptToChat={content.appendPromptToChat}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}
