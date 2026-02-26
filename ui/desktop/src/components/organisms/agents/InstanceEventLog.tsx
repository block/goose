import { ArrowDownToLine, Pause, Terminal } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { InstanceEvent } from '../../../lib/instances';

const eventTypeColors: Record<string, string> = {
  turn_start: 'text-blue-400',
  turn_end: 'text-blue-300',
  tool_call: 'text-purple-400',
  tool_result: 'text-purple-300',
  message: 'text-gray-300',
  error: 'text-red-400',
  status: 'text-cyan-400',
  raw: 'text-gray-500',
};

function formatTimestamp(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

interface InstanceEventLogProps {
  events: InstanceEvent[];
  connected: boolean;
  maxHeight?: string;
}

export function InstanceEventLog({
  events,
  connected,
  maxHeight = '300px',
}: InstanceEventLogProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const userScrolledRef = useRef(false);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 20;

    if (!isAtBottom) {
      userScrolledRef.current = true;
      setAutoScroll(false);
    } else if (userScrolledRef.current) {
      userScrolledRef.current = false;
      setAutoScroll(true);
    }
  };

  const toggleAutoScroll = () => {
    setAutoScroll((prev) => !prev);
    if (!autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  };

  return (
    <div className="rounded-lg border border-border-default overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 bg-background-muted border-b border-border-default">
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <Terminal size={12} />
          <span>Event Stream</span>
          {connected && (
            <span className="flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
              Live
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={toggleAutoScroll}
          className={`p-1 rounded text-xs transition-colors ${
            autoScroll ? 'text-cyan-500 hover:text-cyan-400' : 'text-gray-400 hover:text-gray-300'
          }`}
          title={autoScroll ? 'Pause auto-scroll' : 'Resume auto-scroll'}
        >
          {autoScroll ? <Pause size={12} /> : <ArrowDownToLine size={12} />}
        </button>
      </div>

      {/* Event Log */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="bg-gray-950 font-mono text-xs overflow-y-auto p-3 space-y-0.5"
        style={{ maxHeight }}
      >
        {events.length === 0 ? (
          <div className="text-gray-600 italic">Waiting for events...</div>
        ) : (
          events.map((event) => {
            const colorClass = eventTypeColors[event.type] || 'text-gray-400';
            const stableKey = `${event.timestamp}-${event.type}-${event.data.slice(0, 32)}`;
            return (
              <div key={stableKey} className="flex gap-2 leading-relaxed">
                <span className="text-gray-600 shrink-0">{formatTimestamp(event.timestamp)}</span>
                <span className={`shrink-0 ${colorClass}`}>[{event.type}]</span>
                <span className="text-gray-300 break-all">{event.data}</span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
