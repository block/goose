import { useEffect, useRef } from 'react';
import { Message } from '../api';
import { ChatState } from '../types/chatState';
import { getToolRequests } from '../types/message';
import { useBrowserOptional } from '../components/BrowserContext';

function toolNameToCommand(toolName: string): string | null {
  const mapping: Record<string, string> = {
    browser_open: 'open',
    browser_close: 'close',
    browser_navigate: 'navigate',
    browser_screenshot: 'screenshot',
    browser_click: 'click',
    browser_type: 'type',
    browser_get_text: 'get_text',
    browser_get_html: 'get_html',
    browser_evaluate: 'evaluate',
    browser_wait: 'wait',
    browser_scroll: 'scroll',
  };
  return mapping[toolName] ?? null;
}

/**
 * Watches for forwarded tool requests (tool calls the agent delegated to the client).
 * When the stream finishes and the last assistant message contains a tool request
 * with _meta.forward_to_client, this hook executes it on the browser webview
 * and submits the result back via submitToolResult.
 */
export function useForwardedToolHandler(
  messages: Message[],
  chatState: ChatState,
  submitToolResult: (toolCallId: string, result: string, isError?: boolean) => Promise<void>
) {
  const browser = useBrowserOptional();
  const handledRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (chatState !== ChatState.Idle) return;
    if (!browser) return;

    const lastAssistant = [...messages].reverse().find((m) => m.role === 'assistant');
    if (!lastAssistant) return;

    const toolRequests = getToolRequests(lastAssistant);
    if (!toolRequests.length) return;

    for (const toolRequest of toolRequests) {
      const meta = toolRequest._meta as Record<string, unknown> | undefined;
      if (!meta?.forward_to_client) continue;

      const toolCallId = toolRequest.id;
      if (handledRef.current.has(toolCallId)) continue;
      handledRef.current.add(toolCallId);

      const toolName = toolRequest.toolCall?.name as string | undefined;
      const args = toolRequest.toolCall?.arguments as Record<string, unknown> | undefined;
      const command = toolName ? toolNameToCommand(toolName) : null;

      if (!command || !args) {
        submitToolResult(
          toolCallId,
          JSON.stringify({ error: `Unknown forwarded tool: ${toolName}` }),
          true
        );
        continue;
      }

      (async () => {
        try {
          const result = await browser.executeCommand(command, args);
          await submitToolResult(toolCallId, JSON.stringify(result));
        } catch (err) {
          await submitToolResult(
            toolCallId,
            JSON.stringify({ error: String(err) }),
            true
          );
        }
      })();
    }
  }, [chatState, messages, browser, submitToolResult]);
}
