import { FetchFunction } from '@ai-sdk/provider-utils';
import type {
  Attachment,
  ChatRequest,
  ChatRequestOptions,
  CreateMessage,
  IdGenerator,
  JSONValue,
  Message,
  UseChatOptions,
} from '@ai-sdk/ui-utils';
import { generateId as generateIdFunc } from '@ai-sdk/ui-utils';
import { callCustomChatApi as callChatApi } from './call-custom-chat-api';
import { useCallback, useEffect, useId, useRef, useState } from 'react';
import useSWR, { KeyedMutator } from 'swr';
import { throttle } from './throttle';
import { getSecretKey } from '../config';

export type { CreateMessage, Message, UseChatOptions };

export type UseChatHelpers = {
  /** Current messages in the chat */
  messages: Message[];
  /** The error object of the API request */
  error: undefined | Error;
  /**
   * Append a user message to the chat list. This triggers the API call to fetch
   * the assistant's response.
   * @param message The message to append
   * @param options Additional options to pass to the API call
   */
  append: (
    message: Message | CreateMessage,
    chatRequestOptions?: ChatRequestOptions
  ) => Promise<string | null | undefined>;
  /**
   * Reload the last AI chat response for the given chat history. If the last
   * message isn't from the assistant, it will request the API to generate a
   * new response.
   */
  reload: (chatRequestOptions?: ChatRequestOptions) => Promise<string | null | undefined>;
  /**
   * Abort the current request immediately, keep the generated tokens if any.
   */
  stop: () => void;
  /**
   * Update the `messages` state locally. This is useful when you want to
   * edit the messages on the client, and then trigger the `reload` method
   * manually to regenerate the AI response.
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
  handleSubmit: (
    event?: { preventDefault?: () => void },
    chatRequestOptions?: ChatRequestOptions
  ) => void;
  metadata?: object;
  /** Whether the API request is in progress */
  isLoading: boolean;

  /** Additional data added on the server via StreamData. */
  data?: JSONValue[];
  /** Set the data of the chat. You can use this to transform or clear the chat data. */
  setData: (
    data: JSONValue[] | undefined | ((data: JSONValue[] | undefined) => JSONValue[] | undefined)
  ) => void;
};

const processResponseStream = async (
  api: string,
  chatRequest: ChatRequest,
  mutate: KeyedMutator<Message[]>,
  mutateStreamData: KeyedMutator<JSONValue[] | undefined>,
  existingDataRef: React.MutableRefObject<JSONValue[] | undefined>,
  extraMetadataRef: React.MutableRefObject<any>,
  messagesRef: React.MutableRefObject<Message[]>,
  abortControllerRef: React.MutableRefObject<AbortController | null>,
  generateId: IdGenerator,
  streamProtocol: UseChatOptions['streamProtocol'],
  onFinish: UseChatOptions['onFinish'],
  onResponse: ((response: Response) => void | Promise<void>) | undefined,
  onToolCall: UseChatOptions['onToolCall'] | undefined,
  sendExtraMessageFields: boolean | undefined,
  experimental_prepareRequestBody:
    | ((options: {
        messages: Message[];
        requestData?: JSONValue;
        requestBody?: object;
      }) => JSONValue)
    | undefined,
  fetch: FetchFunction | undefined,
  keepLastMessageOnError: boolean
) => {
  // Do an optimistic update to the chat state to show the updated messages immediately:
  const previousMessages = messagesRef.current;
  mutate(chatRequest.messages, false);

  const constructedMessagesPayload = sendExtraMessageFields
    ? chatRequest.messages
    : chatRequest.messages.map(
        ({ role, content, experimental_attachments, data, annotations, toolInvocations }) => ({
          role,
          content,
          ...(experimental_attachments !== undefined && {
            experimental_attachments,
          }),
          ...(data !== undefined && { data }),
          ...(annotations !== undefined && { annotations }),
          ...(toolInvocations !== undefined && { toolInvocations }),
        })
      );

  const existingData = existingDataRef.current;

  return await callChatApi({
    api,
    body: experimental_prepareRequestBody?.({
      messages: chatRequest.messages,
      requestData: chatRequest.data,
      requestBody: chatRequest.body,
    }) ?? {
      messages: constructedMessagesPayload,
      data: chatRequest.data,
      ...extraMetadataRef.current.body,
      ...chatRequest.body,
    },
    streamProtocol,
    credentials: extraMetadataRef.current.credentials,
    headers: {
      ...extraMetadataRef.current.headers,
      ...chatRequest.headers,
      'X-Secret-Key': getSecretKey(),
    },
    abortController: () => abortControllerRef.current,
    restoreMessagesOnFailure() {
      if (!keepLastMessageOnError) {
        mutate(previousMessages, false);
      }
    },
    onResponse,
    onUpdate(merged, data) {
      mutate([...chatRequest.messages, ...merged], false);
      if (data?.length) {
        mutateStreamData([...(existingData ?? []), ...data], false);
      }
    },
    onToolCall,
    onFinish,
    generateId,
    fetch,
  });
};

export function useChat({
  api = '/api/chat',
  id,
  initialMessages,
  initialInput = '',
  sendExtraMessageFields,
  onToolCall,
  experimental_prepareRequestBody,
  maxSteps = 1,
  streamProtocol = 'data',
  onResponse,
  onFinish,
  onError,
  credentials,
  headers,
  body,
  generateId = generateIdFunc,
  fetch,
  keepLastMessageOnError = true,
  experimental_throttle: throttleWaitMs,
}: UseChatOptions & {
  key?: string;

  /**
   * Experimental (React only). When a function is provided, it will be used
   * to prepare the request body for the chat API. This can be useful for
   * customizing the request body based on the messages and data in the chat.
   *
   * @param messages The current messages in the chat.
   * @param requestData The data object passed in the chat request.
   * @param requestBody The request body object passed in the chat request.
   */
  experimental_prepareRequestBody?: (options: {
    messages: Message[];
    requestData?: JSONValue;
    requestBody?: object;
  }) => JSONValue;

  /**
Custom throttle wait in ms for the chat messages and data updates.
Default is undefined, which disables throttling.
   */
  experimental_throttle?: number;

  /**
Maximum number of sequential LLM calls (steps), e.g. when you use tool calls. Must be at least 1.

A maximum number is required to prevent infinite loops in the case of misconfigured tools.

By default, it's set to 1, which means that only a single LLM call is made.
 */
  maxSteps?: number;
} = {}): UseChatHelpers & {
  addToolResult: ({ toolCallId, result }: { toolCallId: string; result: any }) => void;
} {
  // Generate a unique id for the chat if not provided.
  const hookId = useId();
  const idKey = id ?? hookId;
  const chatKey = typeof api === 'string' ? [api, idKey] : idKey;

  // Store a empty array as the initial messages
  // (instead of using a default parameter value that gets re-created each time)
  // to avoid re-renders:
  const [initialMessagesFallback] = useState([]);

  // Store the chat state in SWR, using the chatId as the key to share states.
  const { data: messages, mutate } = useSWR<Message[]>([chatKey, 'messages'], null, {
    fallbackData: initialMessages ?? initialMessagesFallback,
  });

  // Keep the latest messages in a ref.
  const messagesRef = useRef<Message[]>(messages || []);
  useEffect(() => {
    messagesRef.current = messages || [];
  }, [messages]);

  // stream data
  const { data: streamData, mutate: mutateStreamData } = useSWR<JSONValue[] | undefined>(
    [chatKey, 'streamData'],
    null
  );

  // keep the latest stream data in a ref
  const streamDataRef = useRef<JSONValue[] | undefined>(streamData);
  useEffect(() => {
    streamDataRef.current = streamData;
  }, [streamData]);

  // We store loading state in another hook to sync loading states across hook invocations
  const { data: isLoading = false, mutate: mutateLoading } = useSWR<boolean>(
    [chatKey, 'loading'],
    null
  );

  const { data: error = undefined, mutate: setError } = useSWR<undefined | Error>(
    [chatKey, 'error'],
    null
  );

  // Abort controller to cancel the current API call.
  const abortControllerRef = useRef<AbortController | null>(null);

  const extraMetadataRef = useRef({
    credentials,
    headers,
    body,
  });

  useEffect(() => {
    extraMetadataRef.current = {
      credentials,
      headers,
      body,
    };
  }, [credentials, headers, body]);

  const triggerRequest = useCallback(
    async (chatRequest: ChatRequest) => {
      const messageCount = messagesRef.current.length;

      try {
        mutateLoading(true);
        setError(undefined);

        const abortController = new AbortController();
        abortControllerRef.current = abortController;

        await processResponseStream(
          api,
          chatRequest,
          // throttle streamed ui updates:
          throttle(mutate, throttleWaitMs),
          throttle(mutateStreamData, throttleWaitMs),
          streamDataRef,
          extraMetadataRef,
          messagesRef,
          abortControllerRef,
          generateId,
          streamProtocol,
          onFinish,
          onResponse,
          onToolCall,
          sendExtraMessageFields,
          experimental_prepareRequestBody,
          fetch,
          keepLastMessageOnError
        );

        abortControllerRef.current = null;
      } catch (err) {
        // Ignore abort errors as they are expected.
        if ((err as any).name === 'AbortError') {
          abortControllerRef.current = null;
          return null;
        }

        if (onError && err instanceof Error) {
          onError(err);
        }

        setError(err as Error);
      } finally {
        mutateLoading(false);
      }

      // auto-submit when all tool calls in the last assistant message have results:
      const messages = messagesRef.current;
      const lastMessage = messages[messages.length - 1];
      if (
        // ensure we actually have new messages (to prevent infinite loops in case of errors):
        messages.length > messageCount &&
        // ensure there is a last message:
        lastMessage != null &&
        // check if the feature is enabled:
        maxSteps > 1 &&
        // check that next step is possible:
        isAssistantMessageWithCompletedToolCalls(lastMessage) &&
        // limit the number of automatic steps:
        countTrailingAssistantMessages(messages) < maxSteps
      ) {
        await triggerRequest({ messages });
      }
    },
    [
      mutate,
      mutateLoading,
      api,
      extraMetadataRef,
      onResponse,
      onFinish,
      onError,
      setError,
      mutateStreamData,
      streamDataRef,
      streamProtocol,
      sendExtraMessageFields,
      experimental_prepareRequestBody,
      onToolCall,
      maxSteps,
      messagesRef,
      abortControllerRef,
      generateId,
      fetch,
      keepLastMessageOnError,
      throttleWaitMs,
    ]
  );

  const append = useCallback(
    async (
      message: Message | CreateMessage,
      { data, headers, body, experimental_attachments }: ChatRequestOptions = {}
    ) => {
      if (!message.id) {
        message.id = generateId();
      }

      const attachmentsForRequest = await prepareAttachmentsForRequest(experimental_attachments);

      const messages = messagesRef.current.concat({
        ...message,
        id: message.id ?? generateId(),
        createdAt: message.createdAt ?? new Date(),
        experimental_attachments:
          attachmentsForRequest.length > 0 ? attachmentsForRequest : undefined,
      });

      return triggerRequest({ messages, headers, body, data });
    },
    [triggerRequest, generateId]
  );

  const reload = useCallback(
    async ({ data, headers, body }: ChatRequestOptions = {}) => {
      const messages = messagesRef.current;

      if (messages.length === 0) {
        return null;
      }

      // Remove last assistant message and retry last user message.
      const lastMessage = messages[messages.length - 1];
      return triggerRequest({
        messages: lastMessage.role === 'assistant' ? messages.slice(0, -1) : messages,
        headers,
        body,
        data,
      });
    },
    [triggerRequest]
  );

  const stop = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
  }, []);

  const setMessages = useCallback(
    (messages: Message[] | ((messages: Message[]) => Message[])) => {
      if (typeof messages === 'function') {
        messages = messages(messagesRef.current);
      }

      mutate(messages, false);
      messagesRef.current = messages;
    },
    [mutate]
  );

  const setData = useCallback(
    (
      data: JSONValue[] | undefined | ((data: JSONValue[] | undefined) => JSONValue[] | undefined)
    ) => {
      if (typeof data === 'function') {
        data = data(streamDataRef.current);
      }

      mutateStreamData(data, false);
      streamDataRef.current = data;
    },
    [mutateStreamData]
  );

  // Input state and handlers.
  const [input, setInput] = useState(initialInput);

  const handleSubmit = useCallback(
    async (
      event?: { preventDefault?: () => void },
      options: ChatRequestOptions = {},
      metadata?: object
    ) => {
      event?.preventDefault?.();

      if (!input && !options.allowEmptySubmit) return;

      if (metadata) {
        extraMetadataRef.current = {
          ...extraMetadataRef.current,
          ...metadata,
        };
      }

      const attachmentsForRequest = await prepareAttachmentsForRequest(
        options.experimental_attachments
      );

      const messages =
        !input && !attachmentsForRequest.length && options.allowEmptySubmit
          ? messagesRef.current
          : messagesRef.current.concat({
              id: generateId(),
              createdAt: new Date(),
              role: 'user',
              content: input,
              experimental_attachments:
                attachmentsForRequest.length > 0 ? attachmentsForRequest : undefined,
            });

      const chatRequest: ChatRequest = {
        messages,
        headers: options.headers,
        body: options.body,
        data: options.data,
      };

      triggerRequest(chatRequest);

      setInput('');
    },
    [input, generateId, triggerRequest]
  );

  const handleInputChange = (e: any) => {
    setInput(e.target.value);
  };

  const addToolResult = ({ toolCallId, result }: { toolCallId: string; result: any }) => {
    const updatedMessages = messagesRef.current.map((message, index, arr) =>
      // update the tool calls in the last assistant message:
      index === arr.length - 1 && message.role === 'assistant' && message.toolInvocations
        ? {
            ...message,
            toolInvocations: message.toolInvocations.map((toolInvocation) =>
              toolInvocation.toolCallId === toolCallId
                ? {
                    ...toolInvocation,
                    result,
                    state: 'result' as const,
                  }
                : toolInvocation
            ),
          }
        : message
    );

    mutate(updatedMessages, false);

    // auto-submit when all tool calls in the last assistant message have results:
    const lastMessage = updatedMessages[updatedMessages.length - 1];
    if (isAssistantMessageWithCompletedToolCalls(lastMessage)) {
      triggerRequest({ messages: updatedMessages });
    }
  };

  return {
    messages: messages || [],
    setMessages,
    data: streamData,
    setData,
    error,
    append,
    reload,
    stop,
    input,
    setInput,
    handleInputChange,
    handleSubmit,
    isLoading,
    addToolResult,
  };
}

/**
Check if the message is an assistant message with completed tool calls.
The message must have at least one tool invocation and all tool invocations
must have a result.
 */
function isAssistantMessageWithCompletedToolCalls(message: Message) {
  return (
    message.role === 'assistant' &&
    message.toolInvocations &&
    message.toolInvocations.length > 0 &&
    message.toolInvocations.every((toolInvocation) => 'result' in toolInvocation)
  );
}

/**
Returns the number of trailing assistant messages in the array.
 */
function countTrailingAssistantMessages(messages: Message[]) {
  let count = 0;
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].role === 'assistant') {
      count++;
    } else {
      break;
    }
  }
  return count;
}

async function prepareAttachmentsForRequest(
  attachmentsFromOptions: FileList | Array<Attachment> | undefined
) {
  if (attachmentsFromOptions == null) {
    return [];
  }

  if (attachmentsFromOptions instanceof FileList) {
    return Promise.all(
      Array.from(attachmentsFromOptions).map(async (attachment) => {
        const { name, type } = attachment;

        const dataUrl = await new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = (readerEvent) => {
            resolve(readerEvent.target?.result as string);
          };
          reader.onerror = (error) => reject(error);
          reader.readAsDataURL(attachment);
        });

        return {
          name,
          contentType: type,
          url: dataUrl,
        };
      })
    );
  }

  if (Array.isArray(attachmentsFromOptions)) {
    return attachmentsFromOptions;
  }

  throw new Error('Invalid attachments type');
}
