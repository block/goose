import { useCallback, useEffect, useMemo, useState } from 'react';
import { getApiUrl } from '../config';
import { useMessageStream } from './useMessageStream';
import { fetchSessionDetails } from '../sessions';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import {
  Message,
  createUserMessage,
  ToolCall,
  ToolCallResult,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  ToolConfirmationRequestMessageContent,
  getTextContent,
  TextContent,
} from '../types/message';
import { ChatType } from '../components/hub';

// Helper function to determine if a message is a user message
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
    return false;
  }
  return true;
};

interface UseChatEngineProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  onMessageStreamFinish?: () => void;
  onMessageSent?: () => void; // Add callback for when message is sent
  enableLocalStorage?: boolean;
}

export const useChatEngine = ({
  chat,
  setChat,
  onMessageStreamFinish,
  onMessageSent,
  enableLocalStorage = false,
}: UseChatEngineProps) => {
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [sessionTokenCount, setSessionTokenCount] = useState<number>(0);
  const [ancestorMessages, setAncestorMessages] = useState<Message[]>([]);
  const [sessionInputTokens, setSessionInputTokens] = useState<number>(0);
  const [sessionOutputTokens, setSessionOutputTokens] = useState<number>(0);
  const [localInputTokens, setLocalInputTokens] = useState<number>(0);
  const [localOutputTokens, setLocalOutputTokens] = useState<number>(0);

  // Store message in global history when it's added (if enabled)
  const storeMessageInHistory = useCallback(
    (message: Message) => {
      if (enableLocalStorage && isUserMessage(message)) {
        const text = getTextContent(message);
        if (text) {
          LocalMessageStorage.addMessage(text);
        }
      }
    },
    [enableLocalStorage]
  );

  const {
    messages,
    append: originalAppend,
    stop,
    isLoading,
    error,
    setMessages,
    input: _input,
    setInput: _setInput,
    handleInputChange: _handleInputChange,
    handleSubmit: _submitMessage,
    updateMessageStreamBody,
    notifications,
    sessionMetadata,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    id: chat.id,
    initialMessages: chat.messages,
    body: { session_id: chat.id, session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR') },
    onFinish: async (_message, _reason) => {
      window.electron.stopPowerSaveBlocker();

      const timeSinceLastInteraction = Date.now() - lastInteractionTime;
      window.electron.logInfo('last interaction:' + lastInteractionTime);
      if (timeSinceLastInteraction > 60000) {
        // 60000ms = 1 minute
        window.electron.showNotification({
          title: 'Goose finished the task.',
          body: 'Click here to expand.',
        });
      }

      // Always emit refresh event when message stream finishes for new sessions
      // Check if this is a new session by looking at the current session ID format
      const isNewSession = chat.id && chat.id.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        console.log(
          'ChatEngine: Message stream finished for new session, emitting message-stream-finished event'
        );
        // Emit event to trigger session refresh
        window.dispatchEvent(new CustomEvent('message-stream-finished'));
      }

      onMessageStreamFinish?.();
    },
  });

  // Wrap append to store messages in global history (if enabled)
  const append = useCallback(
    (messageOrString: Message | string) => {
      const message =
        typeof messageOrString === 'string' ? createUserMessage(messageOrString) : messageOrString;
      storeMessageInHistory(message);

      // If this is the first message in a new session, trigger a refresh immediately
      // Only trigger if we're starting a completely new session (no existing messages)
      if (messages.length === 0 && chat.messages.length === 0) {
        // Emit event to indicate a new session is being created
        window.dispatchEvent(new CustomEvent('session-created'));
      }

      return originalAppend(message);
    },
    [originalAppend, storeMessageInHistory, messages.length, chat.messages.length]
  );

  // Simple token estimation function (roughly 4 characters per token)
  const estimateTokens = (text: string): number => {
    return Math.ceil(text.length / 4);
  };

  // Calculate token counts from messages
  useEffect(() => {
    let inputTokens = 0;
    let outputTokens = 0;

    messages.forEach((message) => {
      const textContent = getTextContent(message);
      if (textContent) {
        const tokens = estimateTokens(textContent);
        if (message.role === 'user') {
          inputTokens += tokens;
        } else if (message.role === 'assistant') {
          outputTokens += tokens;
        }
      }
    });

    setLocalInputTokens(inputTokens);
    setLocalOutputTokens(outputTokens);
  }, [messages]);

  // Update chat messages when they change
  useEffect(() => {
    // @ts-expect-error - TypeScript being overly strict about the return type
    setChat((prevChat: ChatType) => ({ ...prevChat, messages }));
  }, [messages, setChat]);

  // Fetch session metadata to get token count
  useEffect(() => {
    const fetchSessionTokens = async () => {
      try {
        const sessionDetails = await fetchSessionDetails(chat.id);
        setSessionTokenCount(sessionDetails.metadata.total_tokens || 0);
        setSessionInputTokens(sessionDetails.metadata.accumulated_input_tokens || 0);
        setSessionOutputTokens(sessionDetails.metadata.accumulated_output_tokens || 0);
      } catch (err) {
        console.error('Error fetching session token count:', err);
      }
    };
    if (chat.id) {
      fetchSessionTokens();
    }
  }, [chat.id, messages]);

  // Update token counts when sessionMetadata changes from the message stream
  useEffect(() => {
    console.log('Session metadata received:', sessionMetadata);
    if (sessionMetadata) {
      setSessionTokenCount(sessionMetadata.totalTokens || 0);
      setSessionInputTokens(sessionMetadata.accumulatedInputTokens || 0);
      setSessionOutputTokens(sessionMetadata.accumulatedOutputTokens || 0);
    }
  }, [sessionMetadata]);

  // Handle submit
  const handleSubmit = useCallback(
    (combinedTextFromInput: string, onSummaryReset?: () => void) => {
      if (combinedTextFromInput.trim()) {
        window.electron.startPowerSaveBlocker();
        setLastInteractionTime(Date.now());

        const userMessage = createUserMessage(combinedTextFromInput.trim());

        if (onSummaryReset) {
          onSummaryReset();
          setTimeout(() => {
            append(userMessage);
            // Call onMessageSent after the message is sent
            onMessageSent?.();
          }, 150);
        } else {
          append(userMessage);
          // Call onMessageSent after the message is sent
          onMessageSent?.();
        }
      } else {
        // If nothing was actually submitted (e.g. empty input and no images pasted)
        window.electron.stopPowerSaveBlocker();
      }
    },
    [append, onMessageSent]
  );

  // Handle stopping the message stream
  const onStopGoose = useCallback(() => {
    stop();
    setLastInteractionTime(Date.now());
    window.electron.stopPowerSaveBlocker();

    // Handle stopping the message stream
    const lastMessage = messages[messages.length - 1];

    // Check if there are any messages before proceeding
    if (!lastMessage) {
      return;
    }

    // check if the last user message has any tool response(s)
    const isToolResponse = lastMessage.content.some(
      (content): content is ToolResponseMessageContent => content.type == 'toolResponse'
    );

    // isUserMessage also checks if the message is a toolConfirmationRequest
    // check if the last message is a real user's message
    if (lastMessage && isUserMessage(lastMessage) && !isToolResponse) {
      // Get the text content from the last message before removing it
      const textContent = lastMessage.content.find((c): c is TextContent => c.type === 'text');
      const textValue = textContent?.text || '';

      // Set the text back to the input field
      _setInput(textValue);

      // Remove the last user message if it's the most recent one
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
    } else if (!isUserMessage(lastMessage)) {
      // the last message was an assistant message
      // check if we have any tool requests or tool confirmation requests
      const toolRequests: [string, ToolCallResult<ToolCall>][] = lastMessage.content
        .filter(
          (content): content is ToolRequestMessageContent | ToolConfirmationRequestMessageContent =>
            content.type === 'toolRequest' || content.type === 'toolConfirmationRequest'
        )
        .map((content) => {
          if (content.type === 'toolRequest') {
            return [content.id, content.toolCall];
          } else {
            // extract tool call from confirmation
            const toolCall: ToolCallResult<ToolCall> = {
              status: 'success',
              value: {
                name: content.toolName,
                arguments: content.arguments,
              },
            };
            return [content.id, toolCall];
          }
        });

      if (toolRequests.length !== 0) {
        // This means we were interrupted during a tool request
        // Create tool responses for all interrupted tool requests

        let responseMessage: Message = {
          display: true,
          sendToLLM: true,
          role: 'user',
          created: Date.now(),
          content: [],
        };

        const notification = 'Interrupted by the user to make a correction';

        // generate a response saying it was interrupted for each tool request
        for (const [reqId, _] of toolRequests) {
          const toolResponse: ToolResponseMessageContent = {
            type: 'toolResponse',
            id: reqId,
            toolResult: {
              status: 'error',
              error: notification,
            },
          };

          responseMessage.content.push(toolResponse);
        }
        // Use an immutable update to add the response message to the messages array
        setMessages([...messages, responseMessage]);
      }
    }
  }, [stop, messages, _setInput, setMessages]);

  // Filter out standalone tool response messages for rendering
  const filteredMessages = useMemo(() => {
    return [...ancestorMessages, ...messages].filter((message) => {
      // Only filter out when display is explicitly false
      if (message.display === false) return false;

      // Keep all assistant messages and user messages that aren't just tool responses
      if (message.role === 'assistant') return true;

      // For user messages, check if they're only tool responses
      if (message.role === 'user') {
        const hasOnlyToolResponses = message.content.every((c) => c.type === 'toolResponse');
        const hasTextContent = message.content.some((c) => c.type === 'text');
        const hasToolConfirmation = message.content.every(
          (c) => c.type === 'toolConfirmationRequest'
        );

        // Keep the message if it has text content or tool confirmation or is not just tool responses
        return hasTextContent || !hasOnlyToolResponses || hasToolConfirmation;
      }

      return true;
    });
  }, [ancestorMessages, messages]);

  // Generate command history from filtered messages
  const commandHistory = useMemo(() => {
    return filteredMessages
      .reduce<string[]>((history, message) => {
        if (isUserMessage(message)) {
          const textContent = message.content.find((c): c is TextContent => c.type === 'text');
          const text = textContent?.text?.trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [filteredMessages]);

  // Process tool call notifications
  const toolCallNotifications = useMemo(() => {
    return notifications.reduce((map, item) => {
      const key = item.request_id;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key).push(item);
      return map;
    }, new Map());
  }, [notifications]);

  return {
    // Core message data
    messages,
    filteredMessages,
    ancestorMessages,
    setAncestorMessages,

    // Message stream controls
    append,
    stop,
    isLoading,
    error,
    setMessages,

    // Input controls
    input: _input,
    setInput: _setInput,
    handleInputChange: _handleInputChange,

    // Event handlers
    handleSubmit,
    onStopGoose,

    // Token and session data
    sessionTokenCount,
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,

    // UI helpers
    commandHistory,
    toolCallNotifications,

    // Stream utilities
    updateMessageStreamBody,
    sessionMetadata,

    // Utilities
    isUserMessage,
  };
};
