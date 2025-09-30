import { useEffect, useState, useRef } from 'react';
import { getApiUrl } from '../config';
import { submitSamplingResponse } from '../api';

// Ensure TextDecoder is available in the global scope
const TextDecoder = globalThis.TextDecoder;

interface SamplingRequest {
  requestId: string;
  extensionName: string;
  messages: Array<{
    role: string;
    content: string;
  }>;
  systemPrompt?: string;
  maxTokens: number;
}

export function useSamplingApprovalSSE() {
  const [currentRequest, setCurrentRequest] = useState<SamplingRequest | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const connectToSamplingApprovalStream = async () => {
      try {
        const abortController = new AbortController();
        abortControllerRef.current = abortController;

        // Use fetch with X-Secret-Key header
        const response = await fetch(getApiUrl('/sampling-approval'), {
          method: 'GET',
          headers: {
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
          signal: abortController.signal,
        });

        if (!response.ok) {
          console.error('Failed to connect to sampling approval stream:', response.statusText);
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
                const request = JSON.parse(data) as SamplingRequest;
                setCurrentRequest(request);
              } catch (error) {
                console.error('Failed to parse sampling request:', error);
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

    connectToSamplingApprovalStream();

    // Cleanup on unmount
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, []);

  const respondToRequest = async (requestId: string, approved: boolean) => {
    try {
      await submitSamplingResponse({
        body: {
          requestId,
          approved,
        },
      });
      setCurrentRequest(null);
    } catch (error) {
      console.error('Failed to submit sampling response:', error);
    }
  };

  const approveRequest = () => {
    if (currentRequest) {
      respondToRequest(currentRequest.requestId, true);
    }
  };

  const denyRequest = () => {
    if (currentRequest) {
      respondToRequest(currentRequest.requestId, false);
    }
  };

  return { currentRequest, approveRequest, denyRequest };
}
