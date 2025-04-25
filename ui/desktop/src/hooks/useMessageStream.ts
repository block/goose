import { useState, useCallback, useEffect, useRef, useId } from 'react';
import useSWR from 'swr';
import { getSecretKey } from '../config';
import { Message, createUserMessage, hasCompletedToolCalls } from '../types/message';

// Ensure TextDecoder is available in the global scope
const TextDecoder = globalThis.TextDecoder;

// Event types for SSE stream
type MessageEvent =
  | { type: 'Message'; message: Message }
  | { type: 'Error'; error: string }
  | { type: 'Finish'; reason: string };

export interface UseMessageStreamOptions {
  /**
   * The API endpoint that accepts a `{ messages: Message[] }` object and returns
   * a stream of messages. Defaults to `/api/chat/reply`.
   */
  api?: string;

  /**
   * A unique identifier for the chat. If not provided, a random one will be
   * generated. When provided, the hook with the same `id` will
   * have shared states across components.
   */
  id?: string;

  /**
   * Initial messages of the chat. Useful to load an existing chat history.
   */
  initialMessages?: Message[];

  /**
   * Initial input of the chat.
   */
  initialInput?: string;

  /**
   * Callback function to be called when a tool call is received.
   * You can optionally return a result for the tool call.
   */
  _onToolCall?: (toolCall: Record<string, unknown>) => void | Promise<unknown> | unknown;

  /**
   * Callback function to be called when the API response is received.
   */
  onResponse?: (response: Response) => void | Promise<void>;

  /**
   * Callback function to be called when the assistant message is finished streaming.
   */
  onFinish?: (message: Message, reason: string) => void;

  /**
   * Callback function to be called when an error is encountered.
   */
  onError?: (error: Error) => void;

  /**
   * HTTP headers to be sent with the API request.
   */
  headers?: Record<string, string> | HeadersInit;

  /**
   * Extra body object to be sent with the API request.
   */
  body?: object;

  /**
   * Maximum number of sequential LLM calls (steps), e.g. when you use tool calls.
   * Default is 1.
   */
  maxSteps?: number;
}

export interface UseMessageStreamHelpers {
  /** Current messages in the chat */
  messages: Message[];

  /** The error object of the API request */
  error: undefined | Error;

  /**
   * Append a user message to the chat list. This triggers the API call to fetch
   * the assistant's response.
   */
  append: (message: Message | string) => Promise<void>;

  /**
   * Reload the last AI chat response for the given chat history.
   */
  reload: () => Promise<void>;

  /**
   * Abort the current request immediately.
   */
  stop: () => void;

  /**
   * Update the `messages` state locally.
   */
  setMessages: (messages: Message[] | ((messages: Message[]) => Message[])) => void;

  /** The current value of the input */
  input: string;

  /** setState-powered method to update the input value */
  setInput: React.Dispatch<React.SetStateAction<string>>;

  /** An input/textarea-ready onChange handler to control the value of the input */
  handleInputChange: (
    e: React.ChangeEvent<HTMLInputElement> | React.ChangeEvent<HTMLTextAreaElement>
  ) => void;

  /** Form submission handler to automatically reset input and append a user message */
  handleSubmit: (event?: { preventDefault?: () => void }) => void;

  /** Whether the API request is in progress */
  isLoading: boolean;

  /** Add a tool result to a tool call */
  addToolResult: ({ toolCallId, result }: { toolCallId: string; result: unknown }) => void;
}

/**
 * Hook for streaming messages directly from the server using the native Goose message format
 */
export function useMessageStream({
  api = '/api/chat/reply',
  id,
  initialMessages = [],
  initialInput = '',
  onResponse,
  onFinish,
  onError,
  headers,
  body,
  maxSteps = 1,
}: UseMessageStreamOptions = {}): UseMessageStreamHelpers {
  // Generate a unique id for the chat if not provided
  const hookId = useId();
  const idKey = id ?? hookId;
  const chatKey = typeof api === 'string' ? [api, idKey] : idKey;

  // Store the chat state in SWR, using the chatId as the key to share states
  const { data: messages, mutate } = useSWR<Message[]>([chatKey, 'messages'], null, {
    fallbackData: initialMessages,
  });

  // Keep the latest messages in a ref
  const messagesRef = useRef<Message[]>(messages || []);
  useEffect(() => {
    messagesRef.current = messages || [];
  }, [messages]);

  // We store loading state in another hook to sync loading states across hook invocations
  const { data: isLoading = false, mutate: mutateLoading } = useSWR<boolean>(
    [chatKey, 'loading'],
    null
  );

  const { data: error = undefined, mutate: setError } = useSWR<undefined | Error>(
    [chatKey, 'error'],
    null
  );

  // Abort controller to cancel the current API call
  const abortControllerRef = useRef<AbortController | null>(null);

  // Extra metadata for requests
  const extraMetadataRef = useRef({
    headers,
    body,
  });

  useEffect(() => {
    extraMetadataRef.current = {
      headers,
      body,
    };
  }, [headers, body]);

  // Process the SSE stream from the server
  const processMessageStream = useCallback(
    async (response: Response, currentMessages: Message[]) => {
      if (!response.body) {
        throw new Error('Response body is empty');
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      try {
        let running = true;
        while (running) {
          const { done, value } = await reader.read();
          if (done) {
            running = false;
            break;
          }

          // Decode the chunk and add it to our buffer
          buffer += decoder.decode(value, { stream: true });

          // Process complete SSE events
          const events = buffer.split('\n\n');
          buffer = events.pop() || ''; // Keep the last incomplete event in the buffer

          for (const event of events) {
            if (event.startsWith('data: ')) {
              try {
                const data = event.slice(6); // Remove 'data: ' prefix
                const parsedEvent = JSON.parse(data) as MessageEvent;

                switch (parsedEvent.type) {
                  case 'Message':
                    // Update messages with the new message
                    currentMessages = [...currentMessages, parsedEvent.message];
                    mutate(currentMessages, false);
                    break;

                  case 'Error':
                    throw new Error(parsedEvent.error);

                  case 'Finish':
                    // Call onFinish with the last message if available
                    if (onFinish && currentMessages.length > 0) {
                      const lastMessage = currentMessages[currentMessages.length - 1];
                      onFinish(lastMessage, parsedEvent.reason);
                    }
                    break;
                }
              } catch (e) {
                console.error('Error parsing SSE event:', e);
                if (onError && e instanceof Error) {
                  onError(e);
                }
              }
            }
          }
        }
      } catch (e) {
        if (e instanceof Error && e.name !== 'AbortError') {
          console.error('Error reading SSE stream:', e);
          if (onError) {
            onError(e);
          }
        }
      } finally {
        reader.releaseLock();
      }

      return currentMessages;
    },
    [mutate, onFinish, onError]
  );

  // Send a request to the server
  const sendRequest = useCallback(
    async (requestMessages: Message[]) => {
      try {
        mutateLoading(true);
        setError(undefined);

        // Create abort controller
        const abortController = new AbortController();
        abortControllerRef.current = abortController;

        // Log request details for debugging
        console.log('Request details:', {
          messages: requestMessages,
          body: extraMetadataRef.current.body,
        });

        // Send request to the server
        const response = await fetch(api, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
            ...extraMetadataRef.current.headers,
          },
          body: JSON.stringify({
            messages: requestMessages,
            ...extraMetadataRef.current.body,
          }),
          signal: abortController.signal,
        });

        if (onResponse) {
          await onResponse(response);
        }

        if (!response.ok) {
          const text = await response.text();
          throw new Error(text || `Error ${response.status}: ${response.statusText}`);
        }

        // Process the SSE stream
        const updatedMessages = await processMessageStream(response, requestMessages);

        // Auto-submit when all tool calls in the last assistant message have results
        if (maxSteps > 1 && updatedMessages.length > requestMessages.length) {
          const lastMessage = updatedMessages[updatedMessages.length - 1];
          if (lastMessage.role === 'assistant' && hasCompletedToolCalls(lastMessage)) {
            // Count trailing assistant messages to prevent infinite loops
            let assistantCount = 0;
            for (let i = updatedMessages.length - 1; i >= 0; i--) {
              if (updatedMessages[i].role === 'assistant') {
                assistantCount++;
              } else {
                break;
              }
            }

            if (assistantCount < maxSteps) {
              await sendRequest(updatedMessages);
            }
          }
        }

        abortControllerRef.current = null;
      } catch (err) {
        // Ignore abort errors as they are expected
        if (err instanceof Error && err.name === 'AbortError') {
          abortControllerRef.current = null;
          return;
        }

        if (onError && err instanceof Error) {
          onError(err);
        }

        setError(err as Error);
      } finally {
        mutateLoading(false);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [api, processMessageStream, mutateLoading, setError, onResponse, onError, maxSteps]
  );

  // Append a new message and send request
  const append = useCallback(
    async (message: Message | string) => {
      // If a string is passed, convert it to a Message object
      const messageToAppend = typeof message === 'string' ? createUserMessage(message) : message;

      console.log('Appending message:', JSON.stringify(messageToAppend, null, 2));

      const currentMessages = [...messagesRef.current, messageToAppend];
      mutate(currentMessages, false);
      await sendRequest(currentMessages);
    },
    [mutate, sendRequest]
  );

  // Reload the last message
  const reload = useCallback(async () => {
    const currentMessages = messagesRef.current;
    if (currentMessages.length === 0) {
      return;
    }

    // Remove last assistant message if present
    const lastMessage = currentMessages[currentMessages.length - 1];
    const messagesToSend =
      lastMessage.role === 'assistant' ? currentMessages.slice(0, -1) : currentMessages;

    await sendRequest(messagesToSend);
  }, [sendRequest]);

  // Stop the current request
  const stop = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
  }, []);

  // Set messages directly
  const setMessages = useCallback(
    (messagesOrFn: Message[] | ((messages: Message[]) => Message[])) => {
      if (typeof messagesOrFn === 'function') {
        const newMessages = messagesOrFn(messagesRef.current);
        mutate(newMessages, false);
        messagesRef.current = newMessages;
      } else {
        mutate(messagesOrFn, false);
        messagesRef.current = messagesOrFn;
      }
    },
    [mutate]
  );

  // Input state and handlers
  const [input, setInput] = useState(initialInput);

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement> | React.ChangeEvent<HTMLTextAreaElement>) => {
      setInput(e.target.value);
    },
    []
  );

  const handleSubmit = useCallback(
    async (event?: { preventDefault?: () => void }) => {
      event?.preventDefault?.();
      if (!input.trim()) return;

      console.log('handleSubmit called with input:', input);
      await append(input);
      setInput('');
    },
    [input, append]
  );

  // Add tool result to a message
  const addToolResult = useCallback(
    ({ toolCallId, result }: { toolCallId: string; result: unknown }) => {
      const currentMessages = messagesRef.current;

      // Find the last assistant message with the tool call
      let lastAssistantIndex = -1;
      for (let i = currentMessages.length - 1; i >= 0; i--) {
        if (currentMessages[i].role === 'assistant') {
          const toolRequests = currentMessages[i].content.filter(
            (content) => content.type === 'toolRequest' && content.id === toolCallId
          );
          if (toolRequests.length > 0) {
            lastAssistantIndex = i;
            break;
          }
        }
      }

      if (lastAssistantIndex === -1) return;

      // Create a tool response message
      const toolResponseMessage: Message = {
        role: 'user' as const,
        created: Math.floor(Date.now() / 1000),
        content: [
          {
            type: 'toolResponse' as const,
            id: toolCallId,
            toolResult: {
              status: 'success' as const,
              value: Array.isArray(result)
                ? result
                : [{ type: 'text' as const, text: String(result), priority: 0 }],
            },
          },
        ],
      };

      // Insert the tool response after the assistant message
      const updatedMessages = [
        ...currentMessages.slice(0, lastAssistantIndex + 1),
        toolResponseMessage,
        ...currentMessages.slice(lastAssistantIndex + 1),
      ];

      mutate(updatedMessages, false);
      messagesRef.current = updatedMessages;

      // Auto-submit if we have tool results
      if (maxSteps > 1) {
        sendRequest(updatedMessages);
      }
    },
    [mutate, maxSteps, sendRequest]
  );

  return {
    messages: messages || [],
    error,
    append,
    reload,
    stop,
    setMessages,
    input,
    setInput,
    handleInputChange,
    handleSubmit,
    isLoading: isLoading || false,
    addToolResult,
  };
}
