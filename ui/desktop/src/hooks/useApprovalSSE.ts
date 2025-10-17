import { useEffect, useState, useRef } from 'react';
import { getApiUrl } from '../config';
import { submitApprovalResponse, ApprovalRequest, ApprovalAction } from '../api';

// Ensure TextDecoder is available in the global scope
const TextDecoder = globalThis.TextDecoder;

export function useApprovalSSE() {
  const [approvalRequest, setCurrentRequest] = useState<ApprovalRequest | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const connectToApprovalStream = async () => {
      try {
        const abortController = new AbortController();
        abortControllerRef.current = abortController;

        // Use fetch with X-Secret-Key header
        const response = await fetch(getApiUrl('/approval'), {
          method: 'GET',
          headers: {
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
          signal: abortController.signal,
        });

        if (!response.ok) {
          console.error('Failed to connect to approval stream:', response.statusText);
          return;
        }

        if (!response.body) {
          console.error('Response body is empty');
          return;
        }

        // Process the SSE stream
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });

          // Process complete SSE events
          const events = buffer.split('\n\n');
          buffer = events.pop() || '';

          for (const event of events) {
            if (event.startsWith('data: ')) {
              const data = event.slice(6); // Remove 'data: ' prefix
              try {
                const request = JSON.parse(data) as ApprovalRequest;
                setCurrentRequest(request);
              } catch (error) {
                console.error('Failed to parse approval request:', error);
              }
            }
          }
        }
      } catch (error) {
        if (error instanceof Error && error.name !== 'AbortError') {
          console.error('SSE connection error:', error);
        }
      }
    };

    connectToApprovalStream();

    // Cleanup on unmount
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, []);

  const respondToRequest = async (requestId: string, action: ApprovalAction) => {
    try {
      await submitApprovalResponse({
        body: {
          requestId,
          action,
        },
      });
      setCurrentRequest(null);
    } catch (error) {
      console.error('Failed to submit approval response:', error);
    }
  };

  const approveOnce = () => {
    if (approvalRequest) {
      const requestId = (approvalRequest as { requestId: string }).requestId;
      respondToRequest(requestId, 'allow_once');
    }
  };

  const approveAlways = () => {
    if (approvalRequest) {
      const requestId = (approvalRequest as { requestId: string }).requestId;
      respondToRequest(requestId, 'always_allow');
    }
  };

  const deny = () => {
    if (approvalRequest) {
      const requestId = (approvalRequest as { requestId: string }).requestId;
      respondToRequest(requestId, 'deny');
    }
  };

  return { approvalRequest, approveOnce, approveAlways, deny, respondToRequest };
}
